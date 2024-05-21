/*!
# Flaca: Images!
*/

mod jpegtran;
pub(super) mod kind;



use kind::ImageKind;
use oxipng::Options as OxipngOptions;
use std::{
	path::Path,
	sync::OnceLock,
};
use super::EncodingError;



static OXIPNG_OPTIONS: OnceLock<OxipngOptions> = OnceLock::new();



#[allow(clippy::inline_always)] // This is the hottest path we've got!
#[inline(always)]
/// # Encode Image.
///
/// This will attempt to losslessly re-encode the image, overriding the
/// original if the compression results in savings.
///
/// The before and after sizes are returned, unless there's an error or the
/// image is invalid. In cases where compression doesn't help, the before and
/// after sizes will be identical.
pub(super) fn encode(file: &Path, kinds: ImageKind)
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
		if ! kinds.supports_png() { return Err(EncodingError::Skipped); }
		encode_oxipng(&mut raw);
		encode_zopflipng(&mut raw);
	}
	// Do JPEG stuff?
	else if ImageKind::is_jpeg(&raw) {
		if ! kinds.supports_jpeg() { return Err(EncodingError::Skipped); }

		// Mozjpeg usually panics on error, so we have to do a weird little
		// dance to keep it from killing the whole thread.
		if let Ok(r) = std::panic::catch_unwind(move || {
			encode_mozjpeg(&mut raw);
			raw
		}) { raw = r; }
		// Abort without changing anything; raw might be tainted.
		else { return Ok((before, before)); }

		// Encoding checks this explicitly, but debug asserts are nothing if
		// not redundant!
		debug_assert!(ImageKind::is_jpeg(&raw));
	}
	// Something else entirely?
	else { return Err(EncodingError::Format); }

	// Save it if better.
	let after = raw.len() as u64;
	if after < before {
		write_atomic::write_file(file, &raw)
			.map(|()| (before, after))
			.map_err(|_| EncodingError::Write)
	}
	else { Ok((before, before)) }
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
	if let Ok(mut new) = oxipng::optimize_from_memory(raw, OXIPNG_OPTIONS.get_or_init(oxipng_options)) {
		if new.len() < raw.len() && ImageKind::is_png(&new) {
			std::mem::swap(raw, &mut new);
		}
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

#[inline(never)]
/// # Generate Oxipng Options.
///
/// This returns the strongest possible Oxipng compression profile (minus
/// the zopfli bits, which we try in a separate pass).
///
/// This is basically just "preset 6", with:
/// * Error fixing enabled;
/// * Libdeflater;
/// * All the alpha optimizations;
/// * Interlacing disabled;
/// * All headers stripped;
fn oxipng_options() -> OxipngOptions {
	use oxipng::{
		Deflaters,
		IndexSet,
		Interlacing,
		RowFilter,
		StripChunks,
	};

	OxipngOptions {
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
}
