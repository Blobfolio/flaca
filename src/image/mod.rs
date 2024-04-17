/*!
# Flaca: Images!
*/

mod jpegtran;
mod ffi;
pub(super) mod kind;
mod lodepng;
mod zopflipng;



use kind::ImageKind;
use oxipng::Options as OxipngOptions;
use std::path::Path;
use zopflipng::{
	deflate_part,
	SplitPoints,
	ZopfliState,
};



/// # Encode Image.
///
/// This will attempt to losslessly re-encode the image, overriding the
/// original if the compression results in savings.
///
/// The before and after sizes are returned, unless there's an error or the
/// image is invalid. In cases where compression doesn't help, the before and
/// after sizes will be identical.
pub(super) fn encode(file: &Path, kinds: ImageKind, oxi: &OxipngOptions) -> Option<(u64, u64)> {
	// Read the file.
	let mut raw = std::fs::read(file).ok()?;
	if raw.is_empty() { return None; }
	let before = u64::try_from(raw.len()).ok()?;

	// Do PNG stuff?
	if ImageKind::is_png(&raw) {
		if ImageKind::None == kinds & ImageKind::Png { return Some((0, 0)); }
		encode_oxipng(&mut raw, oxi);
		encode_zopflipng(&mut raw);
	}
	// Do JPEG stuff?
	else if ImageKind::is_jpeg(&raw) {
		if ImageKind::None == kinds & ImageKind::Jpeg { return Some((0, 0)); }

		// Mozjpeg usually panics on error, so we have to do a weird little
		// dance to keep it from killing the whole thread.
		if let Ok(r) = std::panic::catch_unwind(move || {
			encode_mozjpeg(&mut raw);
			raw
		}) {
			// Copy the data back.
			raw = r;

			// But make sure the copied data didn't get corrupted along the
			// way…
			if ! ImageKind::is_jpeg(&raw) { return Some((before, before)); }
		}
		// Abort without changing anything.
		else { return Some((before, before)); }

	}
	// Something else entirely?
	else { return None; }

	// Save it if better.
	let after = raw.len() as u64;
	if after < before && write_atomic::write_file(file, &raw).is_ok() {
		Some((before, after))
	}
	else { Some((before, before)) }
}

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
		if ImageKind::is_jpeg(slice) {
			raw.truncate(slice.len());
			raw.copy_from_slice(slice);
		}
	}
}

/// # Compress w/ `Oxipng`
///
/// The result is comparable to calling:
///
/// ```bash
/// oxipng -o 3 -s -a -i 0 --fix
/// ```
fn encode_oxipng(raw: &mut Vec<u8>, opts: &OxipngOptions) {
	if let Ok(mut new) = oxipng::optimize_from_memory(raw, opts) {
		if ! new.is_empty() && new.len() < raw.len() && ImageKind::is_png(&new) {
			std::mem::swap(raw, &mut new);
		}
	}
}

/// # Compress w/ `Zopflipng`.
///
/// The result is comparable to calling:
///
/// ```bash
/// zopflipng -m
/// ```
fn encode_zopflipng(raw: &mut Vec<u8>) {
	if let Some(new) = zopflipng::optimize(raw) {
		let slice: &[u8] = &new;
		if ImageKind::is_png(slice) {
			raw.truncate(slice.len());
			raw.copy_from_slice(slice);
		}
	}
}

#[inline]
/// # Generate Oxipng Options.
///
/// This returns the strongest possible Oxipng compression profile (minus
/// the zopfli bits, which we try in a separate pass).
///
/// This is basically just "preset 3", with:
/// * Error fixing enabled;
/// * Libdeflater;
/// * All the alpha optimizations;
/// * Interlacing disabled;
/// * All headers stripped;
pub(super) fn oxipng_options() -> OxipngOptions {
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
