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

use std::os::raw::{
	c_ulong,
	c_uint,
};
use super::lodepng::{
	custom_png_deflate,
	lodepng_color_mode_copy,
	lodepng_compute_color_stats,
	lodepng_is_lct_palette,
	LodePNGColorStats,
	LodePNGColorType_LCT_PALETTE,
	LodePNGColorType_LCT_RGB,
	LodePNGColorType_LCT_RGBA,
	LodePNGDecoder,
	LodePNGEncoder,
	LodePNGFilterStrategy,
	LodePNGFilterStrategy_LFS_ENTROPY,
	LodePNGFilterStrategy_LFS_FOUR,
	LodePNGFilterStrategy_LFS_MINSUM,
	LodePNGFilterStrategy_LFS_ONE,
	LodePNGFilterStrategy_LFS_THREE,
	LodePNGFilterStrategy_LFS_TWO,
	LodePNGFilterStrategy_LFS_ZERO,
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
	let decoded = LodePNGDecoder::new(src)?;

	// Encode!
	let mut buf: Vec<u8> = Vec::new();
	let strategy = best_strategy(&decoded, &mut buf);
	let out_size = try_optimize(&decoded, 32_768, strategy, true, &mut buf);

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
fn best_strategy(dec: &LodePNGDecoder, buf: &mut Vec<u8>) -> LodePNGFilterStrategy {
	let mut size: usize = usize::MAX;
	let mut strategy = LodePNGFilterStrategy_LFS_ZERO;

	// Loop the strategies.
	for s in [
		LodePNGFilterStrategy_LFS_ZERO,
		LodePNGFilterStrategy_LFS_ONE,
		LodePNGFilterStrategy_LFS_TWO,
		LodePNGFilterStrategy_LFS_THREE,
		LodePNGFilterStrategy_LFS_FOUR,
		LodePNGFilterStrategy_LFS_MINSUM,
		LodePNGFilterStrategy_LFS_ENTROPY,
	] {
		// If smaller and nonzero, note which strategy got us there.
		let size2 = try_optimize(dec, 8192, s, false, buf);
		if 0 < size2 && size2 < size {
			size = size2;
			strategy = s;
		}
	}

	strategy
}

#[allow(unsafe_code)]
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
	dec: &LodePNGDecoder,
	window_size: c_uint,
	strategy: LodePNGFilterStrategy,
	use_zopfli: bool,
	buf: &mut Vec<u8>
) -> usize {
	// Start the encoder.
	let mut enc = LodePNGEncoder::default();
	enc.state.encoder.zlibsettings.windowsize = window_size;

	// Copy palette details over to the encoder.
	if dec.state.info_png.color.colortype == LodePNGColorType_LCT_PALETTE {
		// Safety: a non-zero response is an error.
		if 0 != unsafe {
			lodepng_color_mode_copy(&mut enc.state.info_raw, &dec.state.info_png.color)
		} { return 0; }
		enc.state.info_raw.colortype = LodePNGColorType_LCT_RGBA;
		enc.state.info_raw.bitdepth = 8;
	}

	enc.state.encoder.filter_palette_zero = 0;
	enc.state.encoder.filter_strategy = strategy;
	enc.state.encoder.add_id = 0;
	enc.state.encoder.text_compression = 1;

	// For final compression, enable the custom zopfli deflater.
	if use_zopfli {
		enc.state.encoder.zlibsettings.custom_deflate = Some(custom_png_deflate);
		enc.state.encoder.zlibsettings.custom_context = std::ptr::null_mut();
	}

	// Try to encode it.
	let size = enc.encode(dec).map_or(
		0,
		|out| usize::try_from(out.size).map_or(
			0,
			|size| {
				// Copy the output to our buffer.
				buf.resize(size, 0);
				buf.copy_from_slice(unsafe {
					std::slice::from_raw_parts(out.img, size)
				});
				size
			}
		)
	);

	// We might be able to shrink really small output even further.
	if 0 < size && size < 4096 && lodepng_is_lct_palette(buf) {
		let size2 = try_optimize_small(dec, buf, &mut enc);
		if 0 < size2 && size2 < size {
			return size2;
		}
	}

	// Return the number of bytes written.
	size
}

#[allow(clippy::cast_possible_truncation, unsafe_code)]
/// # Apply Optimizations (Small).
///
/// For really small images, space can sometimes be saved by nuking the
/// palette and going with RGB/RGBA instead.
///
/// This will return the number of bytes written if smaller output is produced,
/// otherwise zero.
fn try_optimize_small(dec: &LodePNGDecoder, buf: &mut Vec<u8>, enc: &mut LodePNGEncoder) -> usize {
	// Pull the color stats.
	let mut stats = LodePNGColorStats::default();
	// Safety: a non-zero response is an error.
	if 0 != unsafe {
		lodepng_compute_color_stats(&mut stats, dec.img, dec.w, dec.h, &enc.state.info_raw)
	} { return 0; }

	// The image is small for tRNS chunk overhead.
	if dec.w * dec.h <= 16 && 0 != stats.key {
		stats.alpha = 1;
	}

	// Set the encoding color mode to RGB/RGBA.
	enc.state.encoder.auto_convert = 0;
	enc.state.info_png.color.colortype =
		if 0 == stats.alpha { LodePNGColorType_LCT_RGB }
		else { LodePNGColorType_LCT_RGBA };
	enc.state.info_png.color.bitdepth = 8;

	// Rekey if necessary.
	if 0 == stats.alpha && 0 != stats.key {
		enc.state.info_png.color.key_defined = 1;
		enc.state.info_png.color.key_r = c_uint::from(stats.key_r) & 255;
		enc.state.info_png.color.key_g = c_uint::from(stats.key_g) & 255;
		enc.state.info_png.color.key_b = c_uint::from(stats.key_b) & 255;
	}
	else { enc.state.info_png.color.key_defined = 0; }

	// Try to encode.
	if let Some(out) = enc.encode(dec) {
		if out.size < buf.len() as c_ulong {
			// Copy the content to our buffer!
			let out_size = out.size as usize; // This can't overflow.
			buf.truncate(out_size);
			buf.copy_from_slice(unsafe {
				std::slice::from_raw_parts(out.img, out_size)
			});
			return out_size
		}
	}

	0
}
