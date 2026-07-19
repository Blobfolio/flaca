/*!
# Flaca: Images!
*/

mod gif;
mod jpegtran;
pub(super) mod kind;
mod strip;



use crate::Settings;
use kind::ImageKind;
use std::{
	path::Path,
	sync::OnceLock,
};
use strip::JpegMarker;
use super::EncodingError;



/// # Oxipng Settings.
static OXI: OnceLock<oxipng::Options> = OnceLock::new();



#[expect(clippy::inline_always, reason = "For performance.")]
#[inline(always)]
/// # Encode Image.
///
/// This will attempt to losslessly re-encode the image, overriding the
/// original if the compression results in savings.
///
/// The before and after sizes are returned, unless there's an error or the
/// image is invalid. In cases where compression doesn't help, the before and
/// after sizes will be identical.
pub(super) fn encode(file: &Path, settings: Settings)
-> Result<(u64, u64), EncodingError> {
	// Read the file.
	let mut raw = std::fs::read(file).map_err(|_|
		if file.is_file() { EncodingError::Read }
		else { EncodingError::Vanished }
	)?;
	let before = raw.len() as u64;
	if before == 0 { return Err(EncodingError::Empty); }

	// Do PNG stuff?
	if ImageKind::is_png(&raw) {
		if ! settings.has_kind(ImageKind::Png) { return Err(EncodingError::Skipped); }
		check_resolution(ImageKind::Png, &raw, settings)?;

		encode_oxipng(&mut raw, settings.preserve_meta());
		if settings.zopfli() { encode_zopflipng(&mut raw); }
	}
	// Do JPEG stuff?
	else if ImageKind::is_jpeg(&raw) {
		if ! settings.has_kind(ImageKind::Jpeg) { return Err(EncodingError::Skipped); }
		check_resolution(ImageKind::Jpeg, &raw, settings)?;

		if
			! encode_mozjpeg(&mut raw, settings.preserve_meta()) &&
			! settings.preserve_meta()
		{
			// Second chance to save by stripping metadata!
			strip_jpeg_metadata(&mut raw);
		}
	}
	// Do GIF stuff?
	else if ImageKind::is_gif(&raw) {
		if ! settings.has_kind(ImageKind::Gif) { return Err(EncodingError::Skipped); }
		check_resolution(ImageKind::Gif, &raw, settings)?;

		if
			Some(false) == encode_gif(&mut raw, settings) &&
			! settings.preserve_meta()
		{
			// Second chance to save by stripping metadata!
			strip_gif_metadata(&mut raw);
		}
	}
	// Something else entirely?
	else { return Err(EncodingError::Format); }

	// Save it if better.
	let after = raw.len() as u64;
	if after < before {
		save_image(file, &raw, settings).map(|()| (before, after))
	}
	else { Ok((before, before)) }
}



#[inline(never)]
/// # Check Resolution.
///
/// Parse the image's dimensions and make sure they're within the
/// `MAX_RESOLUTION` runtime constraint.
fn check_resolution(kind: ImageKind, src: &[u8], settings: Settings)
-> Result<(), EncodingError> {
	// Get the width and height.
	let (w, h) = match kind {
		ImageKind::Gif => ImageKind::gif_dimensions(src),
		ImageKind::Jpeg => ImageKind::jpeg_dimensions(src),
		ImageKind::Png => ImageKind::png_dimensions(src),
		_ => None,
	}
		.ok_or(EncodingError::Format)?;

	if settings.check_resolution(w, h) { Ok(()) }
	else { Err(EncodingError::Resolution) }
}

#[inline(never)]
#[must_use]
/// # Recompress GIF.
///
/// The result approximates what programs like `gifsicle` do, but doesn't play
/// quite so fast and loose with the spec.
///
/// Returns `true` if changed.
fn encode_gif(raw: &mut Vec<u8>, settings: Settings) -> Option<bool> {
	let new = gif::optimize(raw, settings)?;
	if
		new.len() < raw.len() &&
		ImageKind::is_gif(&new)
	{
		raw.truncate(new.len());
		raw.copy_from_slice(new.as_slice());
		Some(true)
	}
	else { Some(false) }
}

#[inline(never)]
/// # Strip Metadata Extensions.
///
/// Strip comment/metadata from a GIF, leaving all the other data as-was.
fn strip_gif_metadata(raw: &mut Vec<u8>) {
	if
		let Some(new) = gif::strip_metadata(raw) &&
		new.len() < raw.len() &&
		ImageKind::is_gif(&new)
	{
		raw.truncate(new.len());
		raw.copy_from_slice(&new);
	}
}

#[inline(never)]
#[must_use]
/// # Compress w/ `MozJPEG`.
///
/// The result is comparable to running:
///
/// ```bash
/// jpegtran -copy none -optimize -progressive
/// ```
fn encode_mozjpeg(raw: &mut Vec<u8>, preserve_meta: bool) -> bool {
	if
		let Some(new) = jpegtran::optimize(raw, preserve_meta) &&
		new.len() < raw.len() &&
		ImageKind::is_jpeg(&new)
	{
		raw.truncate(new.len());
		raw.copy_from_slice(&new);
		true
	}
	else { false }
}

#[inline(never)]
/// # Strip Metadata Segments.
///
/// Strip `APP1..=APP13`, `APP15`, and `COM` segments (plus marker padding)
/// from a JPEG, leaving all the other data as-was.
fn strip_jpeg_metadata(raw: &mut Vec<u8>) {
	if
		let Some(new) = JpegMarker::strip_metadata(raw) &&
		new.len() < raw.len() &&
		ImageKind::is_jpeg(&new)
	{
		raw.truncate(new.len());
		raw.copy_from_slice(&new);
	}
}

#[inline(never)]
/// # Compress w/ `Oxipng`
///
/// The result is comparable to calling:
///
/// ```bash
/// oxipng -o 3 -s -a -i 0 --fix
/// ```
fn encode_oxipng(raw: &mut Vec<u8>, preserve_meta: bool) {
	use oxipng::{
		Deflater,
		IndexSet,
		Options,
		FilterStrategy,
		StripChunks,
	};

	let opts = OXI.get_or_init(#[inline(always)] || Options {
		fix_errors: true,
		force: false,
		filters: IndexSet::from([
			FilterStrategy::NONE,
			FilterStrategy::SUB,
			FilterStrategy::UP,
			FilterStrategy::AVERAGE,
			FilterStrategy::PAETH,
			FilterStrategy::MinSum,
			FilterStrategy::Entropy,
			FilterStrategy::Bigrams,
			FilterStrategy::BigEnt,
			FilterStrategy::Brute {
				num_lines: 8,
				level: 5,
			},
		]),
		interlace: Some(false),
		optimize_alpha: true,
		bit_depth_reduction: true,
		color_type_reduction: true,
		palette_reduction: true,
		grayscale_reduction: true,
		idat_recoding: true,
		scale_16: false,
		strip: if preserve_meta { StripChunks::None } else { StripChunks::All },
		deflater: Deflater::Libdeflater { compression: 12 },
		fast_evaluation: false,
		timeout: None,
		max_decompressed_size: None,
	});

	if
		let Ok(mut new) = oxipng::optimize_from_memory(raw, opts) &&
		new.len() < raw.len() &&
		ImageKind::is_png(&new)
	{
		std::mem::swap(raw, &mut new);
	}
}

#[inline(never)]
/// # Compress w/ `Zopflipng`.
///
/// The result is comparable to calling:
///
/// ```bash
/// zopflipng -m
/// ```
fn encode_zopflipng(raw: &mut Vec<u8>) {
	if let Some(new) = flapfli::optimize(raw) {
		let slice: &[u8] = &new;
		if slice.len() < raw.len() && ImageKind::is_png(slice) {
			raw.truncate(slice.len());
			raw.copy_from_slice(slice);
		}
	}
}

/// # Save Image!
fn save_image(src: &Path, data: &[u8], settings: Settings)
-> Result<(), EncodingError> {
	use write_atomic::filetime;
	use write_atomic::filetime::FileTime;

	// Grab the (current) metadata before saving in case the user wants to
	// keep the original file times.
	let times =
		if settings.preserve_times() {
			std::fs::metadata(src).ok().map(|meta| (
				FileTime::from_last_access_time(&meta),
				FileTime::from_last_modification_time(&meta),
			))
		}
		else { None };

	// Save it!
	write_atomic::write_file(src, data).map_err(|_| EncodingError::Write)?;

	// If we have metadata, try to sync the times.
	if let Some((atime, mtime)) = times {
		let _res = filetime::set_file_times(src, atime, mtime);
	}

	Ok(())
}
