/*!
# Flaca: Zopflipng

The `optimize` method in this module emulates the behaviors of the zopflipng
CLI tool when called with:

```bash
zopflipng -m <input> <output>
```

This no longer links to `libzopflipng` itself, but instead reimplements its
functionality.
*/

use std::os::raw::c_uint;
use super::lodepng::{
	DecodedImage,
	LodePNGColorType,
	LodePNGFilterStrategy,
	LodePNGState,
};



/// # Optimize!
///
/// This will attempt to losslessly recompress the source PNG with the
/// strongest Zopfli filter strategy, and return a new PNG image if the result
/// is smaller than the original.
///
/// Note: 16-bit transformations are not lossless; such images will have their
/// bit depths reduced to a more typical 8 bits.
pub(super) fn optimize(src: &[u8]) -> Option<Vec<u8>> {
	let mut dec = LodePNGState::default();
	let img = dec.decode(src)?;

	// Encode!
	let mut buf: Vec<u8> = Vec::new();
	let strategy = best_strategy(&dec, &img, &mut buf);
	let out_size = try_optimize(&dec, &img, 32_768, strategy, true, &mut buf);

	// Return it if better and nonzero!
	if 0 < out_size && out_size < src.len() { Some(buf) }
	else { None }
}



/// # Best Strategy.
///
/// This attempts to find the best filtering strategy for the image by trying
/// all of them in fast mode, and picking whichever produces the smallest
/// output.
///
/// The lodepng `LFS_PREDEFINED` filter strategy is currently unsupported, but
/// that doesn't come up very often, so the "best" selection should still be
/// the actual best in most cases.
fn best_strategy(dec: &LodePNGState, img: &DecodedImage, buf: &mut Vec<u8>) -> LodePNGFilterStrategy {
	let mut size: usize = usize::MAX;
	let mut strategy = LodePNGFilterStrategy::LFS_ZERO;

	// Loop the strategies.
	for s in [
		LodePNGFilterStrategy::LFS_ZERO,
		LodePNGFilterStrategy::LFS_ONE,
		LodePNGFilterStrategy::LFS_TWO,
		LodePNGFilterStrategy::LFS_THREE,
		LodePNGFilterStrategy::LFS_FOUR,
		LodePNGFilterStrategy::LFS_MINSUM,
		LodePNGFilterStrategy::LFS_ENTROPY,
	] {
		// If smaller and nonzero, note which strategy got us there.
		let size2 = try_optimize(dec, img, 8192, s, false, buf);
		if 0 < size2 && size2 < size {
			size = size2;
			strategy = s;
		}
	}

	strategy
}

/// # Apply Optimizations.
///
/// This re-encodes the PNG source using the specified strategy, returning the
/// number of bytes written to the output buffer, or zero if there was an
/// error.
///
/// The `use_zopfli` argument toggles between slow (true) and fast (false)
/// modes. The latter is used to test different strategies, while the former
/// is used for the final output.
fn try_optimize(
	dec: &LodePNGState,
	img: &DecodedImage,
	window_size: c_uint,
	strategy: LodePNGFilterStrategy,
	use_zopfli: bool,
	buf: &mut Vec<u8>
) -> usize {
	// Start the encoder.
	let mut enc = LodePNGState::default();
	enc.encoder.zlibsettings.windowsize = window_size;

	// Copy palette details over to the encoder.
	if dec.info_png.color.colortype == LodePNGColorType::LCT_PALETTE {
		if ! enc.copy_color_mode(dec) { return 0; }
		enc.info_raw.colortype = LodePNGColorType::LCT_RGBA;
		enc.info_raw.bitdepth = 8;
	}

	enc.encoder.filter_palette_zero = 0;
	enc.encoder.filter_strategy = strategy;
	enc.encoder.add_id = 0;
	enc.encoder.text_compression = 1;

	// For final compression, enable the custom zopfli deflater.
	if use_zopfli { enc.use_zopfli(); }

	// Encode and write to the buffer if it worked.
	if let Some(out) = enc.encode(img) {
		buf.truncate(0);
		buf.extend_from_slice(&out);

		// We might be able to save a couple bytes by nuking the palette if the
		// image is already really small.
		if out.size < 4096 && LodePNGColorType::LCT_PALETTE.image_is_type(buf) {
			let size2 = try_optimize_small(img, buf, &mut enc);
			if 0 < size2 && size2 < out.size {
				return size2;
			}
		}

		out.size
	}
	else { 0 }
}

#[allow(clippy::cast_possible_truncation)]
/// # Apply Optimizations (Small).
///
/// For really small images, space can sometimes be saved by nuking the
/// palette and going with RGB/RGBA instead.
///
/// This will return the number of bytes written if smaller output is produced,
/// otherwise zero.
fn try_optimize_small(img: &DecodedImage, buf: &mut Vec<u8>, enc: &mut LodePNGState) -> usize {
	// Pull the color stats.
	let mut stats = match enc.compute_color_stats(img) {
		Some(s) => s,
		None => return 0,
	};

	// The image is small for tRNS chunk overhead.
	if img.w * img.h <= 16 && 0 < stats.key { stats.alpha = 1; }

	// Set the encoding color mode to RGB/RGBA.
	enc.encoder.auto_convert = 0;
	enc.info_png.color.colortype = match (0 < stats.alpha, 0 < stats.colored) {
		(true, true) => LodePNGColorType::LCT_RGBA,
		(false, true) => LodePNGColorType::LCT_RGB,
		(false, false) => LodePNGColorType::LCT_GREY,
		(true, false) => LodePNGColorType::LCT_GREY_ALPHA,
	};
	enc.info_png.color.bitdepth = 8.min(stats.bits);

	// Rekey if necessary.
	if 0 == stats.alpha && 0 < stats.key {
		enc.info_png.color.key_defined = 1;
		enc.info_png.color.key_r = c_uint::from(stats.key_r) & 255;
		enc.info_png.color.key_g = c_uint::from(stats.key_g) & 255;
		enc.info_png.color.key_b = c_uint::from(stats.key_b) & 255;
	}
	else { enc.info_png.color.key_defined = 0; }

	// Encode and write to the buffer if it worked (and is smaller than the
	// previous value).
	if let Some(out) = enc.encode(img) {
		if out.size < buf.len() {
			buf.truncate(out.size);
			buf.copy_from_slice(&out);
			return out.size;
		}
	}

	0
}
