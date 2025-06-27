/*!
# Flaca: Images!
*/

mod jpegtran;
pub(super) mod kind;



use crate::Settings;
use kind::ImageKind;
use std::{
	ffi::CString,
	io::Cursor,
	path::Path,
};
use super::EncodingError;



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

		encode_oxipng(&mut raw);
		encode_zopflipng(&mut raw);
	}
	// Do JPEG stuff?
	else if ImageKind::is_jpeg(&raw) {
		if ! settings.has_kind(ImageKind::Jpeg) { return Err(EncodingError::Skipped); }
		check_resolution(ImageKind::Jpeg, &raw, settings)?;

		// Mozjpeg usually panics on error, so we have to do a weird little
		// dance to keep it from killing the whole thread.
		let raw2 = std::panic::catch_unwind(move || {
			encode_mozjpeg(&mut raw);
			raw
		});

		// Move it back.
		if let Ok(r) = raw2 { raw = r; }
		// Abort without changing anything; raw might be tainted.
		else { return Ok((before, before)); }

		// Encoding checks this explicitly, but debug asserts are nothing if
		// not redundant!
		debug_assert!(ImageKind::is_jpeg(&raw), "BUG: raw was unexpectedly corrupted");
	}
	// Do GIF stuff?
	else if ImageKind::is_gif(&raw) {
		if ! settings.has_kind(ImageKind::Gif) { return Err(EncodingError::Skipped); }
		// The GIF thread will need to handle the actual compression.
		return Err(EncodingError::TbdGif);
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

/// # Encode Image (GIF).
///
/// This will attempt to losslessly re-encode the GIF, overriding the
/// original if the compression results in savings.
///
/// The before and after sizes are returned, unless there's an error or the
/// image is invalid. In cases where compression doesn't help, the before and
/// after sizes will be identical.
pub(super) fn encode_gif(src: &Path, settings: Settings)
-> Result<(u64, u64), EncodingError> {
	// Read the original.
	let mut raw = std::fs::read(src).map_err(|_|
		if src.is_file() { EncodingError::Read }
		else { EncodingError::Vanished }
	)?;
	let before = raw.len() as u64;
	if before == 0 { return Err(EncodingError::Empty); }

	// Check the type and resolution.
	if ! ImageKind::is_gif(&raw) { return Err(EncodingError::Format); }
	check_resolution(ImageKind::Gif, &raw, settings)?;

	encode_image_gif(&mut raw);
	if let Some(new) = encode_gifsicle(src) && new.len() < raw.len() {
		raw.truncate(new.len());
		raw.copy_from_slice(new.as_slice());
	}

	// Save it if better.
	let after = raw.len() as u64;
	if after < before {
		save_image(src, &raw, settings).map(|()| (before, after))
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

/// # Encode w/ `image`.
///
/// Image optimization isn't a particular goal of the `image` crate, but most
/// GIF images are pretty old and shitty and will benefit from a simple in/out,
/// even if just to clear away comments and other metadata.
fn encode_image_gif(raw: &mut Vec<u8>) {
	/// # Try De/Encode.
	fn dec_enc(src: &[u8]) -> Option<Vec<u8>> {
		use image::{
			AnimationDecoder,
			codecs::gif::GifDecoder,
			ImageFormat,
			ImageReader,
		};

		// The image crate's animation-handling isn't robust enough for our
		// purposes, so let's count up the frames and bail if there's more
		// than one.
		if GifDecoder::new(Cursor::new(src)).ok()?.into_frames().take(2).count() != 1 {
			return None;
		}

		// Non-animated GIF frames can have weird settings, so let's start
		// over and use DynamicImage as an agnostic go-between for the in/out
		// pass.
		let img = ImageReader::with_format(Cursor::new(src), ImageFormat::Gif)
			.decode()
			.ok()?;
		let mut new = Cursor::new(Vec::with_capacity(src.len()));
		img.write_to(&mut new, ImageFormat::Gif).ok()?;
		let new = new.into_inner();

		// Return it if non-empty.
		if new.is_empty() { None }
		else { Some(new) }
	}

	// Keep it if better (and still a gif).
	if
		let Some(new) = dec_enc(raw) &&
		new.len() < raw.len() &&
		ImageKind::is_gif(new.as_slice())
	{
		raw.truncate(new.len());
		raw.copy_from_slice(new.as_slice());
	}
}

#[expect(
	clippy::cast_possible_truncation,
	clippy::cast_possible_wrap,
	reason = "False positive.",
)]
#[expect(unsafe_code, reason = "For FFI.")]
/// # Encode w/ `Gifsicle`.
///
/// The result is comparable to running:
///
/// ```bash
/// gifsicle --no-comments SRC --optimize=3
/// ```
///
/// Note: only one instance of this method can be active at any given time.
/// We only call it from a dedicated thread to help with that.
fn encode_gifsicle(src: &Path) -> Option<Vec<u8>> {
	// This unfortunately writes the results to disk, so let's get a
	// tempfile going to help with cleanup.
	let dst = write_atomic::tempfile::Builder::new()
		.prefix(".flaca__")
		.rand_bytes(16)
		.suffix(".gif")
		.tempfile()
		.ok()?;

	// Set up the "args". Note `argv` _has_ to be a `Vec` or gifsicle
	// will fail with a bunch of random not-found errors.
	let args_raw: [CString; 6] = [
		std::env::current_exe().ok()
			.and_then(|c| c.as_os_str().to_str().and_then(|c| CString::new(c).ok()))?,
		CString::new("--no-warnings").ok()?,
		CString::new("--no-comments").ok()?,
		src.as_os_str().to_str().and_then(|c| CString::new(c).ok())?,
		CString::new(format!("--output={}", dst.path().display())).ok()?,
		CString::new("--optimize=3").ok()?,
	];
	let argv = args_raw.iter().map(|v| v.as_ptr()).collect::<Vec<_>>();

	// Safety: zero means success; anything else is grounds for bailing.
	if 0 != unsafe { gifsicle::gifsicle_main(argv.len() as _, argv.as_ptr()) } {
		return None;
	}

	// Read the results and return if not empty (and still a gif).
	let out = std::fs::read(dst).ok()?;
	if out.is_empty() || ! ImageKind::is_gif(&out) { None }
	else { Some(out) }
}

#[inline(never)]
/// # Compress w/ `MozJPEG`.
///
/// The result is comparable to running:
///
/// ```bash
/// jpegtran -copy none -optimize -progressive
/// ```
fn encode_mozjpeg(raw: &mut Vec<u8>) {
	if let Some(new) = jpegtran::optimize(raw) {
		let slice: &[u8] = &new;
		if slice.len() < raw.len() && ImageKind::is_jpeg(slice) {
			raw.truncate(slice.len());
			raw.copy_from_slice(slice);
		}
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
fn encode_oxipng(raw: &mut Vec<u8>) {
	use oxipng::{
		Deflaters,
		IndexSet,
		Interlacing,
		Options,
		RowFilter,
		StripChunks,
	};

	thread_local!(
		static OXI: Options = Options {
			fix_errors: true,
			force: false,
			filter: IndexSet::from([
				RowFilter::None,
				RowFilter::Average,
				RowFilter::BigEnt,
				RowFilter::Bigrams,
				RowFilter::Brute,
				RowFilter::Entropy,
				RowFilter::MinSum,
				RowFilter::Paeth,
				RowFilter::Sub,
				RowFilter::Up,
			]),
			interlace: Some(Interlacing::None),
			optimize_alpha: true,
			bit_depth_reduction: true,
			color_type_reduction: true,
			palette_reduction: true,
			grayscale_reduction: true,
			idat_recoding: true,
			scale_16: false,
			strip: StripChunks::All,
			deflate: Deflaters::Libdeflater { compression: 12 },
			fast_evaluation: false,
			timeout: None,
		}
	);

	if
		let Ok(mut new) = OXI.with(|opts| oxipng::optimize_from_memory(raw, opts)) &&
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
