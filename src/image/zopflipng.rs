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
	c_void,
	c_uchar,
	c_uint,
};
use super::lodepng::{
	custom_png_deflate,
	lodepng_color_mode_copy,
	lodepng_compute_color_stats,
	lodepng_decode,
	lodepng_encode,
	LodePNGColorStats,
	LodePNGColorType_LCT_PALETTE,
	LodePNGColorType_LCT_RGB,
	LodePNGColorType_LCT_RGBA,
	LodePNGFilterStrategy,
	LodePNGFilterStrategy_LFS_ENTROPY,
	LodePNGFilterStrategy_LFS_FOUR,
	LodePNGFilterStrategy_LFS_MINSUM,
	LodePNGFilterStrategy_LFS_ONE,
	LodePNGFilterStrategy_LFS_THREE,
	LodePNGFilterStrategy_LFS_TWO,
	LodePNGFilterStrategy_LFS_ZERO,
	LodePNGState,
};



/// # PNG Decoder.
///
/// This struct holds a `LodePNGState` and the image it decoded. This exists
/// primarily to enforce cleanup on destruction.
struct Decoder {
	state: LodePNGState,
	img: *mut c_uchar,
	w: c_uint,
	h: c_uint,
}

impl Decoder {
	#[allow(unsafe_code)]
	/// # New.
	///
	/// Try to decode a PNG image.
	fn new(src: &[u8]) -> Option<Self> {
		let src_size = c_ulong::try_from(src.len()).ok()?;

		let mut state = LodePNGState::default();
		let mut img = std::ptr::null_mut();
		let mut w = 0;
		let mut h = 0;

		// Safety: a non-zero response is an error.
		let res = unsafe {
			lodepng_decode(&mut img, &mut w, &mut h, &mut state, src.as_ptr(), src_size)
		};

		// Return it if we got it.
		if 0 == res && ! img.is_null() && 0 < w && 0 < h {
			Some(Self { state, img, w, h })
		}
		else { None }
	}
}

impl Drop for Decoder {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if ! self.img.is_null() {
			// Safety: the pointer is non-null.
			unsafe { libc::free(self.img.cast::<c_void>()); }
		}
	}
}



/// # Encoder Image.
///
/// This holds the buffer details for an output image. This exists primarily to
/// enforce cleanup on destruction.
struct EncoderImage {
	img: *mut c_uchar,
	size: c_ulong,
}

impl Default for EncoderImage {
	fn default() -> Self {
		Self {
			img: std::ptr::null_mut(),
			size: 0,
		}
	}
}

impl Drop for EncoderImage {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if ! self.is_empty() {
			// Safety: the pointer is non-null.
			unsafe { libc::free(self.img.cast::<c_void>()); }
		}
	}
}

impl EncoderImage {
	/// # Is Empty?
	///
	/// This returns true if the image has not been initialized.
	fn is_empty(&self) -> bool { 0 == self.size || self.img.is_null() }
}



/// # Optimize!
///
/// This will attempt to losslessly recompress the source PNG with the
/// strongest Zopfli filter strategy, and return a new PNG image if the result
/// is smaller than the original.
///
/// Note: 16-bit transformations are not lossless; such images will have their
/// bit depths reduced to a more typical 8 bits.
pub(super) fn optimize(src: &[u8]) -> Option<Vec<u8>> {
	let decoded = Decoder::new(src)?;

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
fn best_strategy(dec: &Decoder, buf: &mut Vec<u8>) -> LodePNGFilterStrategy {
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
#[inline]
/// # Encode (Wrapper).
fn encode(out: &mut EncoderImage, dec: &Decoder, enc: &mut LodePNGState) -> bool {
	// Safety: a non-zero response is an error.
	let res = unsafe {
		lodepng_encode(&mut out.img, &mut out.size, dec.img, dec.w, dec.h, enc)
	};

	0 == res && ! out.is_empty()
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
	dec: &Decoder,
	window_size: c_uint,
	strategy: LodePNGFilterStrategy,
	use_zopfli: bool,
	buf: &mut Vec<u8>
) -> usize {
	// Start the encoder.
	let mut enc = LodePNGState::default();
	enc.encoder.zlibsettings.windowsize = window_size;

	// Copy palette details over to the encoder.
	if dec.state.info_png.color.colortype == LodePNGColorType_LCT_PALETTE {
		// Safety: a non-zero response is an error.
		if 0 != unsafe {
			lodepng_color_mode_copy(&mut enc.info_raw, &dec.state.info_png.color)
		} { return 0; }
		enc.info_raw.colortype = LodePNGColorType_LCT_RGBA;
		enc.info_raw.bitdepth = 8;
	}

	enc.encoder.filter_palette_zero = 0;
	enc.encoder.filter_strategy = strategy;
	enc.encoder.add_id = 0;
	enc.encoder.text_compression = 1;

	// For final compression, enable the custom zopfli deflater.
	if use_zopfli {
		enc.encoder.zlibsettings.custom_deflate = Some(custom_png_deflate);
		enc.encoder.zlibsettings.custom_context = std::ptr::null_mut();
	}

	// Try to encode it.
	let mut out = EncoderImage::default();
	let size =
		if ! encode(&mut out, dec, &mut enc) { 0 }
		else if let Ok(size) = usize::try_from(out.size) {
			// Copy the output to our buffer.
			buf.resize(size, 0);
			buf.copy_from_slice(unsafe {
				std::slice::from_raw_parts(out.img, size)
			});
			size
		}
		else { 0 };

	// We might be able to shrink really small output even further.
	if 0 < size && size < 4096 {
		if let Some(dec2) = Decoder::new(buf) {
			if dec2.state.info_png.color.colortype == LodePNGColorType_LCT_PALETTE {
				let size2 = try_optimize_small(&dec2, buf, &mut enc);
				if 0 < size2 && size2 < size {
					return size2;
				}
			}
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
fn try_optimize_small(dec: &Decoder, buf: &mut Vec<u8>, enc: &mut LodePNGState) -> usize {
	// Pull the color stats.
	let mut stats = LodePNGColorStats::default();
	// Safety: a non-zero response is an error.
	if 0 != unsafe {
		lodepng_compute_color_stats(&mut stats, dec.img, dec.w, dec.h, &enc.info_raw)
	} { return 0; }

	// The image is small for tRNS chunk overhead.
	if dec.w * dec.h <= 16 && 0 != stats.key {
		stats.alpha = 1;
	}

	// Set the encoding color mode to RGB/RGBA.
	enc.encoder.auto_convert = 0;
	enc.info_png.color.colortype =
		if 0 == stats.alpha { LodePNGColorType_LCT_RGB }
		else { LodePNGColorType_LCT_RGBA };
	enc.info_png.color.bitdepth = 8;

	// Rekey if necessary.
	if 0 == stats.alpha && 0 != stats.key {
		enc.info_png.color.key_defined = 1;
		enc.info_png.color.key_r = c_uint::from(stats.key_r) & 255;
		enc.info_png.color.key_g = c_uint::from(stats.key_g) & 255;
		enc.info_png.color.key_b = c_uint::from(stats.key_b) & 255;
	}
	else { enc.info_png.color.key_defined = 0; }

	// Try to encode.
	let mut out = EncoderImage::default();
	if encode(&mut out, dec, enc) && out.size < buf.len() as c_ulong {
		// Copy the content to our buffer!
		let out_size = out.size as usize; // This can't overflow.
		buf.truncate(out_size);
		buf.copy_from_slice(unsafe {
			std::slice::from_raw_parts(out.img, out_size)
		});
		out_size
	}
	else { 0 }
}
