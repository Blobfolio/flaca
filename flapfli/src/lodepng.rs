/*!
# Flapfli: Lodepng.

This module contains FFI bindings to `lodepng.c`.
*/

#![allow(non_camel_case_types, non_upper_case_globals)]

use std::{
	ffi::{
		c_uchar,
		c_uint,
	},
	mem::MaybeUninit,
};
use super::{
	deflate::flaca_png_deflate,
	EncodedPNG,
	ffi::flapfli_free,
};



// Generated by build.rs.
include!(concat!(env!("OUT_DIR"), "/lodepng-bindgen.rs"));



#[no_mangle]
#[inline(always)]
#[allow(unsafe_code, clippy::inline_always)]
/// # Lodepng CRC32.
///
/// This override allows lodepng to use `crc32fast` for CRC hashing.
///
/// Note: this is more about relative safety than performance; CRC processing
/// times are negligible compared to everything else. Haha.
pub(crate) extern "C" fn lodepng_crc32(buf: *const c_uchar, len: usize) -> c_uint {
	let mut h = crc32fast::Hasher::new();
	h.update(unsafe { std::slice::from_raw_parts(buf, len) });
	h.finalize()
}



#[derive(Debug)]
/// # Decoded Image.
///
/// This is a simple wrapper holding a pointer to a decoded image along with
/// the image dimensions. It enables us to hold one thing instead of three
/// while also ensuring the memory is freed correctly on drop.
pub(super) struct DecodedImage {
	pub(super) buf: *mut c_uchar,
	pub(super) w: c_uint,
	pub(super) h: c_uint,
}

impl Drop for DecodedImage {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		unsafe { flapfli_free(self.buf); }
		self.buf = std::ptr::null_mut();
	}
}

impl Default for LodePNGColorStats {
	#[allow(unsafe_code)]
	fn default() -> Self {
		let mut out = MaybeUninit::<Self>::zeroed();
		unsafe {
			// Safety: lodepng_color_stats_init sets the data.
			lodepng_color_stats_init(out.as_mut_ptr());
			out.assume_init()
		}
	}
}

impl LodePNGColorType {
	/// # Confirm Raw Image Color Type
	///
	/// This reads the color type from the raw image header to check if it
	/// matches `self`.
	///
	/// Note to future self: 25 == 16 (start of IHDR chunk) + 4 (be32 width) + 4
	/// (be32 height) + 1 (bitdepth).
	pub(super) const fn is_match(self, src: &[u8]) -> bool {
		25 < src.len() && src[25] == self as u8
	}
}

impl Default for LodePNGState {
	#[allow(unsafe_code)]
	fn default() -> Self {
		let mut out = MaybeUninit::<Self>::zeroed();
		unsafe {
			// Safety: lodepng_state_init sets the data.
			lodepng_state_init(out.as_mut_ptr());
			out.assume_init()
		}
	}
}

impl Drop for LodePNGState {
	#[allow(unsafe_code)]
	fn drop(&mut self) { unsafe { lodepng_state_cleanup(self) } }
}

impl LodePNGState {
	#[allow(unsafe_code)]
	/// # Decode!
	///
	/// This attempts to decode a raw image byte slice, returning the details
	/// if successful.
	pub(super) fn decode(&mut self, src: &[u8]) -> Option<DecodedImage> {
		let mut buf = std::ptr::null_mut();
		let mut w = 0;
		let mut h = 0;

		// Safety: a non-zero response is an error.
		let res = unsafe {
			lodepng_decode(&mut buf, &mut w, &mut h, self, src.as_ptr(), src.len())
		};

		// Return it if we got it.
		if 0 == res && ! buf.is_null() && 0 != w && 0 != h {
			Some(DecodedImage { buf, w, h })
		}
		else { None }
	}

	#[allow(unsafe_code)]
	/// # Encode!
	///
	/// Encode the image, returning `true` if lodepng was happy and the output
	/// is non-empty.
	pub(super) fn encode(&mut self, img: &DecodedImage, out: &mut EncodedPNG) -> bool {
		// Reset the size.
		out.size = 0;

		// Safety: a non-zero response is an error.
		let res = unsafe {
			lodepng_encode(&mut out.buf, &mut out.size, img.buf, img.w, img.h, self)
		};

		0 == res && ! out.is_null()
	}

	#[allow(unsafe_code)]
	/// # Set Up Encoder.
	///
	/// This configures and returns a new state for general encoding purposes.
	/// As this is recycled across runs, separate methods are used to configure
	/// the strategy and zopfliness.
	pub(super) fn encoder(dec: &Self) -> Option<Self> {
		let mut enc = Self::default();

		// Copy palette details over to the encoder.
		if dec.info_png.color.colortype == LodePNGColorType::LCT_PALETTE {
			// Safety: a non-zero response indicates an error.
			if 0 != unsafe {
				lodepng_color_mode_copy(&mut enc.info_raw, &dec.info_png.color)
			} { return None; }

			enc.info_raw.colortype = LodePNGColorType::LCT_RGBA;
			enc.info_raw.bitdepth = 8;
		}

		enc.encoder.filter_palette_zero = 0;
		enc.encoder.filter_strategy = LodePNGFilterStrategy::LFS_ZERO;
		enc.encoder.zlibsettings.windowsize = 8_192;

		Some(enc)
	}

	/// # Change Strategies.
	pub(super) fn set_strategy(&mut self, strategy: LodePNGFilterStrategy) {
		self.encoder.filter_strategy = strategy;
	}

	/// # Prepare for Zopfli.
	///
	/// Increase the window size and enable our custom zopfli deflate callback.
	/// For performance reasons, this is only called before the final
	/// encoding pass; everything else is run with saner tunings.
	pub(super) fn set_zopfli(&mut self) {
		self.encoder.zlibsettings.windowsize = 32_768;
		self.encoder.zlibsettings.custom_deflate = Some(flaca_png_deflate);
	}

	#[allow(unsafe_code)]
	#[inline(never)]
	/// # Paletteless Encode (for small images).
	///
	/// Patch the encoder settings to see if we can squeeze even more savings
	/// out of the (small) image, reencode it, and return the result if there
	/// are no errors.
	///
	/// Note: the caller will need to check the resulting size to see if
	/// savings were actually achieved, and keep whichever version was better.
	pub(super) fn try_small(&mut self, img: &DecodedImage) -> Option<EncodedPNG> {
		// Safety: a non-zero response is an error.
		let mut stats = LodePNGColorStats::default();
		if 0 != unsafe {
			lodepng_compute_color_stats(&mut stats, img.buf, img.w, img.h, &self.info_raw)
		} { return None; }

		// The image is too small for tRNS chunk overhead.
		if img.w * img.h <= 16 && 0 != stats.key { stats.alpha = 1; }

		// Set the encoding color mode to RGB/RGBA.
		self.encoder.auto_convert = 0;
		self.info_png.color.colortype = match (0 != stats.colored, 0 != stats.alpha) {
			(true, false) => LodePNGColorType::LCT_RGB,
			(true, true) => LodePNGColorType::LCT_RGBA,
			(false, false) => LodePNGColorType::LCT_GREY,
			(false, true) => LodePNGColorType::LCT_GREY_ALPHA,
		};
		self.info_png.color.bitdepth = u32::min(8, stats.bits);

		// Rekey if necessary.
		if 0 == stats.alpha && 0 != stats.key {
			self.info_png.color.key_defined = 1;
			self.info_png.color.key_r = c_uint::from(stats.key_r) & 255;
			self.info_png.color.key_g = c_uint::from(stats.key_g) & 255;
			self.info_png.color.key_b = c_uint::from(stats.key_b) & 255;
		}
		else { self.info_png.color.key_defined = 0; }

		// Re-encode it and see what happens!
		let mut out = EncodedPNG::new();
		if self.encode(img, &mut out) { Some(out) }
		else { None }
	}
}



#[cfg(test)]
#[allow(deref_nullptr, non_snake_case, trivial_casts, unsafe_code)]
mod tests {
	use super::*;

	#[test]
	fn t_color_type_is_match() {
		for (p, t) in [
			("../skel/assets/png/01.png", LodePNGColorType::LCT_RGB),
			("../skel/assets/png/02.png", LodePNGColorType::LCT_RGBA),
			("../skel/assets/png/04.png", LodePNGColorType::LCT_GREY),
			("../skel/assets/png/small-bwa.png", LodePNGColorType::LCT_GREY_ALPHA),
		] {
			let raw = match std::fs::read(p) {
				Ok(x) => x,
				_ => panic!("Missing {}", p),
			};
			assert!(t.is_match(&raw));
		}

		// Let's test a negative to make sure we aren't doing something silly.
		let raw = std::fs::read("../skel/assets/png/01.png").unwrap();
		assert!(! LodePNGColorType::LCT_GREY.is_match(&raw));
	}

	// Generated by build.rs (layout tests).
	include!(concat!(env!("OUT_DIR"), "/lodepng-bindgen-tests.rs"));
}