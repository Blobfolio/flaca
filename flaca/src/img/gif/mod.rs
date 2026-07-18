/*!
# Flaca: GIF Work.
*/

mod lzw;

use dactyl::NoHash;
use gif::{
	AnyExtension,
	ColorOutput,
	DecodeOptions,
	DisposalMethod,
	Encoder,
	Frame,
	Repeat,
};
use gif_dispose::Screen;
use std::{
	borrow::Cow,
	collections::{
		BTreeSet,
		HashMap,
	},
	hash,
	io::Cursor,
	num::{
		NonZeroU16,
		NonZeroUsize,
	},
};



/// # Extension Label and Block(s).
type ExtensionLabelAndBlocks<'a> = (AnyExtension, Vec<&'a [u8]>);

/// # Max Palette Size.
const MAX_COLOR_TABLE: usize = 256;

/// # Non-Zero Two.
const NZ2: NonZeroUsize = NonZeroUsize::new(2).unwrap();

/// # Static Hasher.
///
/// This is used for cheap collision detection. No need to get fancy with it.
const AHASHER: ahash::RandomState = ahash::RandomState::with_seeds(
	0x8596_cc44_bef0_1aa0,
	0x98d4_0948_da60_19ae,
	0x49f1_3013_c503_a6aa,
	0xc4d7_82ff_3c9f_7bef,
);



/// # Optimize Gif.
///
/// This method attempts to optimize a GIF image by:
/// * Stripping metadata (unless `preserve_meta`)
/// * Stripping unused palette colors
/// * Sorting/merging palettes
/// * Inter-frame blit/delta fuckery
/// * Exhaustive LZW
pub(super) fn optimize(src: &[u8], preserve_meta: bool) -> Option<Vec<u8>> {
	// First pass.
	let decoded = DecodedGif::new(src)?;
	if decoded.frames.is_empty() {
		std::hint::cold_path();
		return None;
	}

	// Metadata?
	let meta = if preserve_meta { find_extensions(src) } else { None };

	let mut cache = FrameCache::new(decoded.frames.len());

	// Encode it a few different ways, keeping whichever copy comes out best.
	Palette::global_palettes(decoded.frames.iter().map(|v| &v.palette)).iter()
		.map(Some)
		.chain(std::iter::once(None))
		.filter_map(|g| decoded.encode(g, meta.as_deref(), None, &mut cache))
		.min_by(|a, b| a.len().cmp(&b.len()))
}

#[must_use]
/// # Strip Metadata.
///
/// Remove metadata from a GIF _without_ re-encoding it, returning a copy of
/// the image if smaller.
///
/// Metadata _shouldn't_ normally impact rendering for a GIF, but just in case
/// the before/after images are decoded and compared before yielding.
pub(super) fn strip_metadata(src: &[u8]) -> Option<Vec<u8>> {
	/// # Same Frame?
	fn same_frame(a: &Frame, b: &Frame) -> bool {
		a.delay == b.delay &&
		a.dispose == b.dispose &&
		a.transparent == b.transparent &&
		a.needs_user_input == b.needs_user_input &&
		a.top == b.top &&
		a.left == b.left &&
		a.width == b.width &&
		a.height == b.height &&
		a.interlaced == b.interlaced &&
		a.palette == b.palette &&
		a.buffer == b.buffer
	}

	// Get a stripped copy.
	let new = strip_extensions(src)?;
	if new.is_empty() || src.len() < new.len() { return None; }

	// Decode source.
	let mut opts = DecodeOptions::new();
	opts.set_color_output(ColorOutput::Indexed);
	let mut dec_a = opts.read_info(Cursor::new(src)).ok()?;

	// Decode copy.
	let mut opts = DecodeOptions::new();
	opts.set_color_output(ColorOutput::Indexed);
	let mut dec_b = opts.read_info(Cursor::new(new.as_slice())).ok()?;

	// Confirm dimensions and repeat settings are the same for both.
	if
		dec_a.width() != dec_b.width() ||
		dec_a.height() != dec_b.height() ||
		dec_a.repeat() != dec_b.repeat()
	{
		return None;
	}

	// Because we didn't re-encode anything, both copies should yield identical
	// frames. (Otherwise we'd have to composite to be sure.)
	while let Some(frame_a) = dec_a.read_next_frame().ok()? {
		let frame_b = dec_b.read_next_frame().ok()??;
		if ! same_frame(frame_a, frame_b) {
			return None;
		}
	}

	// We're good unless B has extra frames…
	if dec_b.read_next_frame().ok()?.is_none() { Some(new) }
	else { None }
}



#[derive(Debug)]
/// # First Pass.
///
/// This struct is used to hold all the relevant deconstructed image data
/// required for (eventual) encoding.
struct DecodedGif {
	/// # Width.
	width: NonZeroU16,

	/// # Height.
	height: NonZeroU16,

	/// # Repeat.
	repeat: Repeat,

	/// # Frames.
	frames: Vec<ProtoFrame>,
}

impl DecodedGif {
	#[must_use]
	/// # New.
	///
	/// Decode the image, building up per-frame palettes and settings,
	/// returning the results if successful.
	fn new(src: &[u8]) -> Option<Self> {
		// Set up the decoder.
		let mut opts = DecodeOptions::new();
		opts.set_color_output(ColorOutput::Indexed);
		let mut dec = opts.read_info(Cursor::new(src)).ok()?;
		let width = NonZeroU16::new(dec.width())?;
		let height = NonZeroU16::new(dec.height())?;
		let repeat = dec.repeat();

		// Parse the frames.
		let mut screen = Screen::new_decoder(&dec);
		let mut last_canvas: Option<Vec<PixelColor>> = None;
		let mut frames: Vec<ProtoFrame> = Vec::new();
		while let Some(frame) = dec.read_next_frame().ok()? {
			screen.blit_frame(frame).ok()?;

			// Normalize transparent pixels.
			let canvas: Vec<PixelColor> = screen.pixels_rgba()
				.pixels()
				.map(PixelColor::from_rgba8)
				.collect();

			// If any of the composited pixels in this frame are newly
			// transparent, or this frame contains no overlap with the last,
			// change the disposal method of the previous frame to
			// "background".
			if
				let Some(last) = last_canvas.as_ref() &&
				(
					canvas.iter().zip(last.iter()).any(|(b, a)| b.is_transparent() && ! a.is_transparent()) ||
					canvas.iter().zip(last.iter()).all(|(b, a)| a != b)
				)
			{
				frames.last_mut()?.dispose = DisposalMethod::Background;
				last_canvas = None;
			}

			// One more time around.
			let frame =
				if let Some(last) = last_canvas.take() {
					ProtoFrame::new_composite(
						width,
						height,
						frame.delay,
						frame.needs_user_input,
						&canvas,
						&last,
					)?
				}
				else {
					ProtoFrame::new(
						width,
						height,
						frame.delay,
						frame.needs_user_input,
						&canvas,
					)
				};

			last_canvas = Some(canvas);
			frames.push(frame);
		}

		// Finish it up!
		if frames.is_empty() { None }
		else {
			Some(Self { width, height, repeat, frames })
		}
	}
}

impl DecodedGif {
	#[must_use]
	/// # Encode GIF.
	///
	/// Combine all the parts into a new GIF image, optionally using the
	/// provided global palette and/or metadata.
	fn encode(
		&self,
		global_palette: Option<&Palette>,
		meta: Option<&[ExtensionLabelAndBlocks]>,
		alignment: Option<usize>,
		cache: &mut FrameCache,
	) -> Option<Vec<u8>> {
		// Set up the encoder.
		let mut enc = Encoder::new(
			Vec::with_capacity(
				usize::from(self.width.get()) *
				usize::from(self.height.get())
			),
			self.width.get(),
			self.height.get(),
			&if let Some(g) = global_palette { g.flatten()? }
			else { Vec::new() },
		).ok()?;
		enc.set_repeat(self.repeat).ok()?;

		// Write metadata, if any.
		if let Some(meta) = meta {
			for (label, blocks) in meta {
				enc.write_raw_extension(*label, blocks).ok()?;
			}
		}

		// Write the frames.
		for frame in &self.frames {
			let frame = frame.try_into_frame(global_palette, alignment, cache)?;
			enc.write_lzw_pre_encoded_frame(&frame).ok()?;
		}

		// Done!
		let out = enc.into_inner().ok()?;
		if out.is_empty() { None }
		else { Some(out) }
	}
}



#[derive(Debug, Clone, Copy)]
/// # Bounding Box.
///
/// This struct represents the draw area for a single frame in a GIF. For
/// animations, this is often less than the full image area.
struct MinMaxXY {
	/// # X Range (Inclusive).
	x: (u16, u16),

	/// # Y Range (Inclusive).
	y: (u16, u16),
}

impl MinMaxXY {
	/// # Max Coordinate.
	const MAX_COORDINATE: usize = u16::MAX as usize;

	/// # Max Index.
	const MAX_IDX: usize = Self::MAX_COORDINATE.checked_mul(Self::MAX_COORDINATE).unwrap();

	/// # New.
	fn new(
		current: &[PixelColor],
		last: &[PixelColor],
		width: NonZeroU16,
	) -> Option<Self> {
		let mut min_x = u16::MAX;
		let mut max_x = u16::MIN;
		let mut min_y = u16::MAX;
		let mut max_y = u16::MIN;

		// We're working with composited values, so this shouldn't ever fail.
		if Self::MAX_IDX < current.len() || current.len() != last.len() {
			std::hint::cold_path();
			return None;
		}

		// Find the first and last difference for each axis.
		for (i, (b, a)) in current.iter().zip(last.iter()).enumerate() {
			if a != b {
				let (x, y) = Self::index_to_xy(i, width)?;

				min_x = u16::min(min_x, x);
				max_x = u16::max(max_x, x);

				min_y = u16::min(min_y, y);
				max_y = u16::max(max_y, y);
			}
		}

		// Done!
		Some(Self {
			x: (min_x, max_x),
			y: (min_y, max_y),
		})
	}

	#[must_use]
	/// # Is Empty?
	///
	/// The ranges are inclusive, so "empty" is represented internally by
	/// min/max being in the wrong order.
	const fn is_empty(self) -> bool {
		self.x.1 < self.x.0 ||
		self.y.1 < self.y.0
	}

	#[must_use]
	/// # Left Position.
	const fn left(self) -> u16 { self.x.0 }

	#[must_use]
	/// # Top Position.
	const fn top(self) -> u16 { self.y.0 }

	#[must_use]
	/// # Width.
	const fn width(self) -> Option<NonZeroU16> {
		if
			let Some(sub) = self.x.1.checked_sub(self.x.0) &&
			let Some(add) = sub.checked_add(1)
		{
			NonZeroU16::new(add)
		}
		else { None }
	}

	#[must_use]
	/// # Height.
	const fn height(self) -> Option<NonZeroU16> {
		if
			let Some(sub) = self.y.1.checked_sub(self.y.0) &&
			let Some(add) = sub.checked_add(1)
		{
			NonZeroU16::new(add)
		}
		else { None }
	}

	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	#[must_use]
	/// # Index to Coordinates.
	const fn index_to_xy(idx: usize, width: NonZeroU16) -> Option<(u16, u16)> {
		if Self::MAX_IDX < idx {
			std::hint::cold_path();
			None
		}
		else {
			let width = width.get() as usize;
			let x = (idx % width) as u16;
			let y = (idx / width) as u16;
			Some((x, y))
		}
	}

	#[must_use]
	/// # Contains.
	///
	/// Returns a bool indicating if a given pixel falls within the bounding
	/// box, or `None` if that pixel is impossibly big.
	const fn contains(self, idx: usize, width: NonZeroU16) -> Option<bool> {
		if let Some((x, y)) = Self::index_to_xy(idx, width) {
			Some(
				self.x.0 <= x && x <= self.x.1 &&
				self.y.0 <= y && y <= self.y.1
			)
		}
		else { None }
	}
}



#[derive(Debug, Clone, Eq, Hash, Ord, PartialEq, PartialOrd)]
/// # Color Palette.
///
/// This struct holds a unique list of RGB colors.
struct Palette(Vec<PixelColor>);

impl Default for Palette {
	#[inline]
	fn default() -> Self {
		Self(Vec::with_capacity(MAX_COLOR_TABLE))
	}
}

impl From<PixelColor> for Palette {
	#[inline]
	fn from(px: PixelColor) -> Self {
		let mut out = Self(Vec::with_capacity(MAX_COLOR_TABLE));
		out.0.push(px);
		out
	}
}

impl Palette {
	/// # Push Color.
	fn push(&mut self, px: PixelColor) {
		if let Err(idx) = self.0.binary_search(&px) { self.0.insert(idx, px); }
	}

	#[must_use]
	/// # Contains All?
	fn contains_all(&self, other: &Self) -> bool {
		self.len() >= other.len() &&
		other.0.iter().copied().all(|px| self.contains(px))
	}

	#[must_use]
	/// # Length.
	const fn len(&self) -> usize { self.0.len() }

	#[must_use]
	/// # Contains.
	fn contains(&self, px: PixelColor) -> bool {
		self.0.binary_search(&px).is_ok()
	}

	#[must_use]
	/// # Flatten.
	fn flatten(&self) -> Option<Vec<u8>> {
		if MAX_COLOR_TABLE < self.len() {
			std::hint::cold_path();
			None
		}
		else {
			let mut out = Vec::with_capacity(self.len() * 3);
			for px in self.0.iter().copied() {
				out.extend_from_slice(&match px {
					PixelColor::Transparent => self.transparent_rgb(),
					PixelColor::Rgb(v) => v,
				});
			}
			Some(out)
		}
	}

	#[must_use]
	/// # Color Look Up.
	///
	/// Return the index for the color, if any.
	fn lookup(&self, px: PixelColor) -> Option<u8> {
		self.0.binary_search(&px).ok().and_then(|v| u8::try_from(v).ok())
	}

	#[must_use]
	/// # Transparent Color.
	fn transparent_rgb(&self) -> [u8; 3] {
		// Try grayscale first.
		for c in u8::MIN..=u8::MAX {
			let px = PixelColor::Rgb([c, c, c]);
			if ! self.contains(px) { return [c, c, c]; }
		}

		// Try 'em all.
		for r in u8::MIN..=u8::MAX {
			for g in u8::MIN..=u8::MAX {
				for b in u8::MIN..=u8::MAX {
					let px = PixelColor::Rgb([r, g, b]);
					if ! self.contains(px) { return [r, g, b]; }
				}
			}
		}

		// We shouldn't ever get here!
		[0, 0, 0]
	}

	#[must_use]
	/// # Transparent Index.
	fn transparent_idx(&self) -> Option<u8> { self.lookup(PixelColor::Transparent) }
}

impl Palette {
	/// # Global Palette(s).
	///
	/// Merge individual frame palettes together into a single global palette,
	/// or multiple global palettes if they don't all fit.
	///
	/// All returned palettes are sorted by color.
	fn global_palettes<'a, I: Iterator<Item=&'a Self>>(palettes: I) -> Vec<Self> {
		/// # Merge Two Palettes.
		///
		/// Merge `other` into `self` if the total remains under the limit,
		/// returning `true` if successful.
		fn merge_one(base: &mut Palette, other: &Palette) -> bool {
			let len_base = base.len();
			let mut diff = Vec::with_capacity(other.len());
			for px in other.0.iter().copied() {
				if ! base.contains(px) { diff.push(px); }
			}

			let len_diff = diff.len();
			if len_base + len_diff <= MAX_COLOR_TABLE {
				if len_diff != 0 {
					base.0.extend_from_slice(&diff);
					base.0.sort();
				}
				true
			}
			else { false }
		}

		/// # Merge Multiple Palettes.
		///
		/// Try to merge one or more palettes into the first, returning the
		/// result along with the number of frames included, or `None` if no
		/// mergers happened.
		fn merge_many(src: &[&Palette]) -> Option<(Palette, usize)> {
			let mut iter = src.iter().copied();
			let mut out = iter.next()?.clone();
			let mut included = 1;

			for next in iter {
				if merge_one(&mut out, next) { included += 1; }
			}

			// Return if there was a merger.
			if 1 < included { Some((out, included)) }
			else { None }
		}

		// Collect the palettes.
		let mut palettes: Vec<&Self> = palettes
			.filter(|v| v.len() <= MAX_COLOR_TABLE)
			.collect();

		// Size-based short circuits.
		match palettes.len() {
			// Empty?
			0 => {
				std::hint::cold_path();
				Vec::new()
			},

			// A single frame can be passed back as-is.
			1 => vec![palettes[0].clone()],

			// Two frames can only be combined one way, so return both or
			// neither.
			2 => {
				let mut out = palettes[0].clone();
				if merge_one(&mut out, palettes[1]) { vec![out] }
				else { Vec::new() }
			}

			// Three or more frames might result in multiple combinations.
			_ => {
				// Potential mergers.
				let mut out = BTreeSet::new();

				// Start with the frames ordered smallest to largest.
				palettes.sort_by_key(|v| v.len());
				for _ in 0..palettes.len() {
					if let Some((maybe, n)) = merge_many(&palettes) {
						if n == palettes.len() { return vec![maybe]; }
						out.insert(maybe);
					}

					// Rotate and back around again.
					palettes.rotate_left(1);
				}

				// Reverse (largest to smallest) and repeat.
				palettes.reverse();
				for _ in 0..palettes.len() {
					if let Some((maybe, _)) = merge_many(&palettes) {
						out.insert(maybe);
					}
					palettes.rotate_left(1);
				}

				// Return 'em if we got 'em.
				out.into_iter().collect()
			},
		}
	}
}



#[derive(Debug, Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
/// # Pixel Color.
///
/// GIF image palettes are 3-byte, with transparent called out separately.
enum PixelColor {
	/// # Transparent.
	Transparent,

	/// # RGB.
	Rgb([u8; 3]),
}

impl hash::Hash for PixelColor {
	#[inline]
	fn hash<H: hash::Hasher>(&self, state: &mut H) {
		state.write_u32(match self {
			Self::Transparent => 0,
			Self::Rgb([r, g, b]) => u32::from_le_bytes([*r, *g, *b, 1]),
		});
	}
}

impl PixelColor {
	/*
	#[must_use]
	/// # From RGB8.
	const fn from_rgb8(px: gif_dispose::RGB8) -> Self {
		Self::Rgb([px.r, px.g, px.b])
	}
	*/

	#[must_use]
	/// # From RGBA8.
	const fn from_rgba8(px: gif_dispose::RGBA8) -> Self {
		if px.a == 0 { Self::Transparent }
		else { Self::Rgb([px.r, px.g, px.b]) }
	}

	#[must_use]
	/// # Is Transparent?
	const fn is_transparent(self) -> bool { matches!(self, Self::Transparent) }
}



#[derive(Debug, Clone)]
/// # Proto Frame.
struct ProtoFrame {
	/// # Animation: Delay.
	delay: u16,

	/// # Animation: Disposal Method.
	dispose: DisposalMethod,

	/// # Dunno.
	needs_user_input: bool,

	/// # Start Y.
	top: u16,

	/// # Start X.
	left: u16,

	/// # Width.
	width: NonZeroU16,

	/// # Height.
	height: NonZeroU16,

	/// # Palette.
	palette: Palette,

	/// # Blitted Pixels.
	canvas: Vec<PixelColor>,
}

impl ProtoFrame {
	#[must_use]
	/// # New (Standalone).
	fn new(
		width: NonZeroU16,
		height: NonZeroU16,
		delay: u16,
		needs_user_input: bool,
		canvas: &[PixelColor],
	) -> Self {
		let mut palette = Palette::default();
		for px in canvas { palette.push(*px); }

		Self {
			delay,
			dispose: DisposalMethod::Keep,
			needs_user_input,
			top: 0,
			left: 0,
			width,
			height,
			palette,
			canvas: canvas.to_vec(),
		}
	}

	#[must_use]
	/// # New (Standalone).
	fn new_composite(
		width: NonZeroU16,
		height: NonZeroU16,
		delay: u16,
		needs_user_input: bool,
		current: &[PixelColor],
		last: &[PixelColor],
	) -> Option<Self> {
		// Draw a box around what's different.
		let bounds = MinMaxXY::new(current, last, width)?;

		// If nothing whatsoever is, submit the frame as a single transparent
		// pixel.
		if bounds.is_empty() {
			return Some(Self {
				delay,
				dispose: DisposalMethod::Keep,
				needs_user_input,
				top: 0,
				left: 0,
				width: NonZeroU16::MIN,
				height: NonZeroU16::MIN,
				palette: Palette::from(PixelColor::Transparent),
				canvas: vec![PixelColor::Transparent],
			});
		}

		// Map the bounds to the canvas.
		let top = bounds.top();
		let left = bounds.left();
		let bounds_width = bounds.width()?;
		let bounds_height = bounds.height()?;
		if width < bounds_width || height < bounds_height { return None; }

		// Build up a map of the (relevant) pixels.
		let mut palette = Palette::default();
		let mut canvas = Vec::with_capacity(
			usize::from(bounds_width.get()) *
			usize::from(bounds_height.get())
		);
		for (i, (b, a)) in current.iter().zip(last.iter()).enumerate() {
			if bounds.contains(i, width)? {
				if a == b {
					palette.push(PixelColor::Transparent);
					canvas.push(PixelColor::Transparent);
				}
				else {
					palette.push(*b);
					canvas.push(*b);
				}
			}
		}

		Some(Self {
			delay,
			dispose: DisposalMethod::Keep,
			needs_user_input,
			top,
			left,
			width: bounds_width,
			height: bounds_height,
			palette,
			canvas,
		})
	}
}

impl ProtoFrame {
	#[must_use]
	/// # Recommended LZW Alignment.
	///
	/// Return a high-but-not-too-high alignment value for the number of pixels
	/// comprising the frame.
	const fn recommended_alignment(&self) -> NonZeroUsize {
		// Align not-small images to the row size.
		if
			(u16::MAX as usize) < self.canvas.len() &&
			let Some(alignment) = NonZeroUsize::new(self.width.get() as usize)
		{
			alignment
		}
		// Align small ones to every other byte.
		else if (i16::MAX as usize) < self.canvas.len() { NZ2 }
		// Tiny images can be tested exhaustively.
		else { NonZeroUsize::MIN }
	}

	#[must_use]
	/// # Into Frame.
	fn try_into_frame(
		&self,
		mut global_palette: Option<&Palette>,
		alignment: Option<usize>,
		cache: &mut FrameCache,
	) -> Option<Frame<'static>> {
		// Global is only global if it covers all of our colors.
		if global_palette.is_some_and(|v| ! v.contains_all(&self.palette)) {
			global_palette = None;
		}

		let alignment = alignment.map_or_else(
			|| Some(self.recommended_alignment()),
			NonZeroUsize::new
		);

		// Build up an indexed buffer.
		let ref_palette = global_palette.unwrap_or(&self.palette);
		let mut buffer: Vec<u8> = Vec::with_capacity(self.canvas.len());
		for px in self.canvas.iter().copied() {
			buffer.push(ref_palette.lookup(px)?);
		}

		// Try LZW our way!
		let lzw = alignment.and_then(|v| cache.encode_frame(&buffer, v));

		// Palette for the frame.
		let palette =
			if global_palette.is_none() { Some(self.palette.flatten()?) }
			else { None };

		// The frame.
		let mut out = Frame {
			delay: self.delay,
			dispose: self.dispose,
			transparent: ref_palette.transparent_idx(),
			needs_user_input: self.needs_user_input,
			top: self.top,
			left: self.left,
			width: self.width.get(),
			height: self.height.get(),
			palette,
			interlaced: false,
			buffer: Cow::Owned(buffer),
		};
		out.make_lzw_pre_encoded();

		// Swap LZW if ours is better.
		if let Some(lzw) = lzw && lzw.len() < out.buffer.len() {
			lzw.clone_into(out.buffer.to_mut());
		}

		// Done!
		Some(out)
	}
}



/// # LZW Frame Cache.
///
/// Images are potentially encoded multiple times to test different palette
/// configurations, but those changes don't always affect the color indices
/// compressed by LZW.
///
/// This cache ensures we don't waste time calculating the same thing twice.
struct FrameCache(HashMap<u64, Option<Vec<u8>>, NoHash>);

impl FrameCache {
	#[must_use]
	/// # New Instance.
	fn new(capacity: usize) -> Self {
		Self(HashMap::with_capacity_and_hasher(capacity, NoHash::default()))
	}

	#[must_use]
	/// # Encode Frame.
	///
	/// Return an LZW-encoded canvas for the frame, whether new or cached.
	fn encode_frame<'a>(
		&'a mut self,
		data: &[u8],
		alignment: NonZeroUsize,
	) -> Option<&'a [u8]> {
		use std::collections::hash_map::Entry;

		let key = AHASHER.hash_one((alignment, data));
		match self.0.entry(key) {
			Entry::Vacant(e) => e.insert(lzw::encode_frame(data, alignment)).as_deref(),
			Entry::Occupied(e) => e.into_mut().as_deref(),
		}
	}
}



#[must_use]
/// # Find Metadata Extensions.
///
/// The `gif` decoder skips miscellaneous application/metadata blocks, which is
/// fine, except when it isn't.
///
/// This method manually extracts and returns any such affected blocks, or
/// `None` if there are any parsing issues.
fn find_extensions(src: &[u8]) -> Option<Vec<ExtensionLabelAndBlocks<'_>>> {
	// The first six bytes are magic.
	let (magic, src) = src.split_first_chunk::<6>()?;
	if ! matches!(magic, [b'G', b'I', b'F', b'8', b'7' | b'9', b'a']) {
		return None;
	}

	// The next seven bytes give us width (2), height (2), flags, background
	// index, and pixel aspect ratio. After that is usually a global color
	// table, but to know if and how big, we have to parse the flags.
	let flags = *src.get(4)?;
	let mut pos = 7;
	if flags & 0x80 != 0 {
		pos += 3 * (1 << ((flags & 0x07) + 1));
	}

	// From here, it's block by block.
	let mut out = Vec::new();
	loop {
		match *src.get(pos)? {
			// Trailer. There shouldn't be anything after this.
			0x3B => break,

			// Image stuff.
			0x2C => {
				// The frame might have its own color table. As with the
				// header, we have to parse the flags to know if and how big
				// it is to skip past it.
				let flags = *src.get(pos + 9)?;
				pos += 10;
				if flags & 0x80 != 0 {
					pos += 3 * (1 << ((flags & 0x07) + 1));
				}

				// LZW data follows, beginning with the minimum code size.
				pos += 1;
				loop {
					let len = usize::from(*src.get(pos)?);
					pos += 1;
					if len == 0 { break; }
					pos += len;
				}
			},

			// Extension block.
			0x21 => {
				let label = *src.get(pos + 1)?;
				pos += 2;

				// The decoder handles graphics control extensions.
				let mut ignore = label == 0xF9;

				let mut blocks = Vec::new();
				while pos < src.len() {
					let len = usize::from(src[pos]);
					pos += 1;
					if len == 0 { break; }

					let block = src.get(pos..pos + len)?;

					// The decoder also handles netscape/animation.
					if
						! ignore &&
						label == 0xFF &&
						blocks.is_empty() &&
						matches!(
							block.first_chunk::<11>(),
							Some(b"ANIMEXTS1.0" | b"NETSCAPE2.0"),
						)
					{
						ignore = true;
					}

					if ! ignore { blocks.push(block); }
					pos += len;
				}

				// Add it if we got it.
				if ! blocks.is_empty() {
					out.push((AnyExtension(label), blocks));
				}
			},

			// Dunno?
			_ => return None,
		}
	}

	// Return it if we got it.
	if out.is_empty() { None }
	else { Some(out) }
}

#[must_use]
/// # Strip Metadata Extensions.
///
/// This method builds upa  copy of the original source image _without_ any
/// comment/metadata extensions, i.e. what `find_extensions` would return.
///
/// Returns `None` if the image is unchanged or cannot be parsed.
fn strip_extensions(src: &[u8]) -> Option<Vec<u8>> {
	// The first six bytes are magic.
	if ! matches!(
		src.first_chunk::<6>(),
		Some([b'G', b'I', b'F', b'8', b'7' | b'9', b'a']),
	) {
		return None;
	}
	let mut pos = 6;

	// The next seven bytes give us width (2), height (2), flags, background
	// index, and pixel aspect ratio. After that is usually a global color
	// table, but to know if and how big, we have to parse the flags.
	let flags = *src.get(pos + 4)?;
	pos += 7;
	if flags & 0x80 != 0 {
		pos += 3 * (1 << ((flags & 0x07) + 1));
	}

	// From here, it's block by block.
	let mut out = Vec::with_capacity(src.len());
	out.extend_from_slice(src.get(..pos)?);
	loop {
		// Note the starting point.
		let from = pos;
		match *src.get(pos)? {
			// Trailer. There shouldn't be anything after this.
			0x3B => {
				// Copy the rest of the data, then we're done!
				out.extend_from_slice(&src[pos..]);
				break
			},

			// Image stuff.
			0x2C => {
				// The frame might have its own color table. As with the
				// header, we have to parse the flags to know if and how big
				// it is to skip past it.
				let flags = *src.get(pos + 9)?;
				pos += 10;
				if flags & 0x80 != 0 {
					pos += 3 * (1 << ((flags & 0x07) + 1));
				}

				// LZW data follows, beginning with the minimum code size.
				pos += 1;
				loop {
					let len = usize::from(*src.get(pos)?);
					pos += 1;
					if len == 0 { break; }
					pos += len;
				}

				// Copy the whole chunk.
				out.extend_from_slice(src.get(from..pos)?);
			},

			// Extension block.
			0x21 => {
				let label = *src.get(pos + 1)?;
				pos += 2;

				// Keep graphics control and animation extensions.
				let mut keep =
					if label == 0xF9 { Some(true) }
					else { None };

				while pos < src.len() {
					let len = usize::from(src[pos]);
					pos += 1;
					if len == 0 { break; }

					let block = src.get(pos..pos + len)?;
					if keep.is_none() {
						keep = Some(
							label == 0xFF &&
							matches!(
								block.first_chunk::<11>(),
								Some(b"ANIMEXTS1.0" | b"NETSCAPE2.0"),
							)
						);
					}
					pos += len;
				}

				// Add it if we got it.
				if keep != Some(false) {
					out.extend_from_slice(src.get(from..pos)?);
				}
			},

			// Dunno?
			_ => return None,
		}
	}

	// Return it if we got it.
	if out.is_empty() || out.len() == src.len() { None }
	else { Some(out) }
}
