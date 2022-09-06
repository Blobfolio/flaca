/*!
# Flaca: Images!
*/

mod jpegtran;
pub(self) mod ffi;
pub(super) mod kind;
pub(self) mod lodepng;
mod zopflipng;



use kind::ImageKind;
use oxipng::Options as OxipngOptions;
use std::path::Path;



/// # Encode Image.
///
/// This will attempt to losslessly re-encode the image, overriding the
/// original if the compression results in savings.
///
/// The before and after sizes are returned, unless there's an error or the
/// image is invalid. In cases where compression doesn't help, the before and
/// after sizes will be identical.
pub(super) fn encode(file: &Path, kinds: u8, oxi: &OxipngOptions) -> Option<(u64, u64)> {
	// Read the file.
	let mut raw = std::fs::read(file).ok()?;
	if raw.is_empty() { return None; }
	let before = u64::try_from(raw.len()).ok()?;

	// Do PNG stuff?
	if ImageKind::is_png(&raw) {
		if 0 == kinds & ImageKind::Png { return None; }
		encode_oxipng(&mut raw, oxi);
		encode_zopflipng(&mut raw);
	}
	// Do JPEG stuff?
	else if ImageKind::is_jpeg(&raw) {
		if 0 == kinds & ImageKind::Jpeg { return None; }
		encode_mozjpeg(&mut raw);
	}
	// Bad image.
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
		AlphaOptim,
		Deflaters,
		Headers,
		IndexSet,
	};

	OxipngOptions {
		backup: false,
		fix_errors: true,
		check: false,
		pretend: false,
		force: false,
		preserve_attrs: false,
		filter: IndexSet::from([0, 1, 2, 3, 4, 5]),
		interlace: Some(0),
		alphas: IndexSet::from([
			AlphaOptim::NoOp,
			AlphaOptim::Black, AlphaOptim::Down, AlphaOptim::Left,
			AlphaOptim::Right, AlphaOptim::Up, AlphaOptim::White,
		]),
		bit_depth_reduction: true,
		color_type_reduction: true,
		palette_reduction: true,
		grayscale_reduction: true,
		idat_recoding: true,
		strip: Headers::All,
		deflate: Deflaters::Libdeflater,
		use_heuristics: false,
		timeout: None,
	}
}
