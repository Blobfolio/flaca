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
use indexmap::IndexSet;
use std::{
	borrow::Cow,
	cmp::Ordering,
	collections::BTreeMap,
	hash,
	io::Cursor,
	num::NonZeroU16,
};



/// # Extension Label and Block(s).
type ExtensionLabelAndBlocks<'a> = (AnyExtension, Vec<&'a [u8]>);



/// # Optimize Gif.
///
/// This method attempts to optimize a GIF image by:
/// * Stripping metadata (unless `preserve_meta`)
/// * Optimizing and/or merging color table(s)
/// * Inter-frame blit/delta fuckery
/// * Exhaustive LZW
///
/// Programs like `gifsicle` can usually achieve greater savings by appealing
/// to "realworld" practice — assumed behaviors, etc. — but that's kinda
/// dangerous, so we don't do that. Haha.
pub(super) fn optimize(src: &[u8], preserve_meta: bool) -> Option<Vec<u8>> {
	// First pass.
	let decoded = DecodedGif::new(src)?;
	if decoded.frames.is_empty() {
		std::hint::cold_path();
		return None;
	}

	// Metadata?
	let meta = if preserve_meta { find_extensions(src) } else { None };

	// Encode it a few different ways, keeping whichever copy is best.
	Palette::global_palettes(decoded.frames.iter()).iter()
		.map(Some)
		.chain(std::iter::once(None))
		.filter_map(|g| decoded.encode(g, meta.as_deref()))
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
		meta: Option<&[ExtensionLabelAndBlocks]>
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
			let frame = frame.try_into_frame(global_palette)?;
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



#[derive(Debug, Clone, Eq, PartialEq)]
/// # Color Palette.
///
/// This struct holds a unique list of RGB colors.
struct Palette(IndexSet<PixelColor, NoHash>);

impl Default for Palette {
	#[inline]
	fn default() -> Self {
		Self(IndexSet::with_capacity_and_hasher(256, NoHash::default()))
	}
}

impl From<PixelColor> for Palette {
	#[inline]
	fn from(px: PixelColor) -> Self {
		let mut out = Self(IndexSet::with_capacity_and_hasher(1, NoHash::default()));
		out.0.insert(px);
		out
	}
}

impl hash::Hash for Palette {
	#[inline]
	fn hash<H: hash::Hasher>(&self, state: &mut H) {
		for px in &self.0 { px.hash(state); }
	}
}

impl Palette {
	/// # Push Color.
	fn push(&mut self, px: PixelColor) { self.0.insert(px); }

	/// # Sort Palette.
	fn sort(&mut self) { self.0.sort(); }

	#[must_use]
	/// # Contains All?
	fn contains_all(&self, other: &Self) -> bool {
		other.0.iter().all(|px| self.0.contains(px))
	}

	#[must_use]
	/// # Length.
	fn len(&self) -> usize { self.0.len() }

	#[must_use]
	/// # Flatten.
	fn flatten(&self) -> Option<Vec<u8>> {
		if 256 < self.len() { None }
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
		self.0.get_index_of(&px).and_then(|v| u8::try_from(v).ok())
	}

	#[must_use]
	/// # Transparent Color.
	fn transparent_rgb(&self) -> [u8; 3] {
		// Try grayscale first.
		for c in u8::MIN..=u8::MAX {
			let px = PixelColor::Rgb([c, c, c]);
			if ! self.0.contains(&px) { return [c, c, c]; }
		}

		// Try 'em all.
		for r in u8::MIN..=u8::MAX {
			for g in u8::MIN..=u8::MAX {
				for b in u8::MIN..=u8::MAX {
					let px = PixelColor::Rgb([r, g, b]);
					if ! self.0.contains(&px) { return [r, g, b]; }
				}
			}
		}

		// We shouldn't ever get here!
		[0, 0, 0]
	}

	#[must_use]
	/// # Transparent Index.
	fn transparent_idx(&self) -> Option<u8> {
		self.0.get_index_of(&PixelColor::Transparent).and_then(|v| u8::try_from(v).ok())
	}
}

impl Palette {
	/// # Global Palette(s).
	///
	/// Merge the frame palette(s) by size — biggest to smallest and vice
	/// versa — sorted by color, pixel frequency, and frame coverage.
	///
	/// The number of possibilities actually returned will vary by image.
	fn global_palettes<'a, I: Iterator<Item=&'a ProtoFrame>>(frames: I) -> Vec<Self> {
		/// # Insert Variations.
		fn insert_variations(
			mut palette: Palette,
			set: &mut IndexSet<Palette>,
			count: usize,
			frame_freqs: &BTreeMap<PixelColor, usize>,
			pixel_freqs: &BTreeMap<PixelColor, usize>,
		) {
			// By color.
			set.insert(palette.clone());

			// Sort by frame frequency, if we have more than one.
			if 1 < count {
				palette.0.sort_by(|a, b| match frame_freqs.get(b).cmp(&frame_freqs.get(a)) {
					Ordering::Equal => a.cmp(b),
					cmp => cmp,
				});
				set.insert(palette.clone());
			}

			// Sort by pixel frequency.
			palette.0.sort_by(|a, b| match pixel_freqs.get(b).cmp(&pixel_freqs.get(a)) {
				Ordering::Equal => a.cmp(b),
				cmp => cmp,
			});
			set.insert(palette);
		}

		// Collect the palettes and counts.
		let mut palettes = Vec::new();
		let mut pixel_freqs = BTreeMap::<PixelColor, usize>::new();
		let mut frame_freqs = BTreeMap::<PixelColor, usize>::new();
		for v in frames {
			// Impossible frames won't encode so fuck 'em.
			if 256 < v.palette.len() { return Vec::new(); }

			palettes.push(&v.palette);
			for px in v.palette.0.iter().copied() {
				*(frame_freqs.entry(px).or_default()) += 1;
			}
			for px in v.canvas.iter().copied() {
				*(pixel_freqs.entry(px).or_default()) += 1;
			}
		}

		// Possibilities.
		let mut out = IndexSet::new();
		match palettes.len().cmp(&1) {
			// Noop.
			Ordering::Less => {},

			// Resort the main palette a few ways.
			Ordering::Equal => {
				insert_variations(
					palettes[0].clone(),
					&mut out,
					1,
					&frame_freqs,
					&pixel_freqs,
				);
			},

			// Merge by pallete size.
			Ordering::Greater =>  if let Some((palette, n)) = Self::global_merged(&palettes, false) {
				insert_variations(palette, &mut out, n, &frame_freqs, &pixel_freqs);

				// If anything got left out, reverse the order.
				if
					n < palettes.len() &&
					let Some((palette, n)) = Self::global_merged(&palettes, true)
				{
					insert_variations(palette, &mut out, n, &frame_freqs, &pixel_freqs);
				}
			},
		}

		// Return whatever we've got.
		out.into_iter().collect()
	}

	/// # Global Palette: Merged by Palette Size.
	fn global_merged(src: &[&Self], asc: bool) -> Option<(Self, usize)> {
		if src.len() < 2 { return None; }
		let mut src: Vec<&Self> = src.to_vec();

		// Smallest to biggest.
		if asc { src.sort_by_key(|v| v.len()); }
		// Biggest to smallest.
		else { src.sort_by_key(|v| std::cmp::Reverse(v.len())); }

		// Start with the highest.
		let mut iter = src.iter().copied();
		let mut out = iter.next()?.clone();
		let mut included = 1;

		// Merge 'em if we got 'em.
		for next in iter {
			if Self::try_merge(&mut out, next) { included += 1; }
		}

		// Shouldn't be possible to go over, but let's double-check before
		// suggesting it!
		if 256 < out.len() { None }
		else {
			out.sort();
			Some((out, included))
		}
	}

	/// # Try Merge.
	///
	/// Merge `other` into `self` if the total remains under the limit,
	/// returning `true` if successful.
	fn try_merge(base: &mut Self, other: &Self) -> bool {
		let len_base = base.len();
		let diff = other.0.difference(&base.0)
			.copied()
			.collect::<Vec<PixelColor>>();

		if len_base + diff.len() <= 256 {
			base.0.extend(diff);
			true
		}
		else { false }
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
		palette.sort();

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
		palette.sort();

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
	/// # Into Frame.
	fn try_into_frame(&self, mut global_palette: Option<&Palette>)
	-> Option<Frame<'static>> {
		// Global is only global if it covers all of our colors.
		if global_palette.is_some_and(|v| ! v.contains_all(&self.palette)) {
			global_palette = None;
		}

		// Build up an indexed buffer.
		let ref_palette = global_palette.unwrap_or(&self.palette);
		let mut buffer: Vec<u8> = Vec::with_capacity(self.canvas.len());
		for px in self.canvas.iter().copied() {
			buffer.push(ref_palette.lookup(px)?);
		}

		// Try LZW our way!
		let lzw = lzw::encode_frame(&buffer);

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
