/*!
# Flaca: Lodepng

These are FFI bindings to the upstream `lodepng.c` library functionality
required to recreate the `zopflipng` behaviors in Rust.

The `custom_png_deflate` extern is not part of lodepng, but gets attached to
`LodePNGState` instances, so is included here as well.
*/

#![allow(non_camel_case_types, non_upper_case_globals)]

use std::{
	ffi::{
		c_char,
		c_uchar,
		c_uint,
		c_ushort,
		c_void,
	},
	mem::MaybeUninit,
};
use super::ffi::EncodedImage;



#[derive(Debug)]
/// # Decoded Image.
pub(super) struct DecodedImage {
	pub(super) buf: *mut c_uchar,
	pub(super) w: c_uint,
	pub(super) h: c_uint,
}

impl Drop for DecodedImage {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if ! self.buf.is_null() {
			unsafe { libc::free(self.buf.cast::<c_void>()); }
			self.buf = std::ptr::null_mut();
		}
	}
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGColorMode {
	pub(super) colortype: LodePNGColorType,
	pub(super) bitdepth: c_uint,
	pub(super) palette: *mut c_uchar,
	pub(super) palettesize: usize,
	pub(super) key_defined: c_uint,
	pub(super) key_r: c_uint,
	pub(super) key_g: c_uint,
	pub(super) key_b: c_uint,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGColorStats {
	pub(super) colored: c_uint,
	pub(super) key: c_uint,
	pub(super) key_r: c_ushort,
	pub(super) key_g: c_ushort,
	pub(super) key_b: c_ushort,
	pub(super) alpha: c_uint,
	pub(super) numcolors: c_uint,
	pub(super) palette: [c_uchar; 1024usize],
	pub(super) bits: c_uint,
	pub(super) numpixels: usize,
	pub(super) allow_palette: c_uint,
	pub(super) allow_greyscale: c_uint,
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

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(super) enum LodePNGColorType {
	LCT_GREY = 0,
	LCT_RGB = 2,
	LCT_PALETTE = 3,
	LCT_GREY_ALPHA = 4,
	LCT_RGBA = 6,
	// LCT_MAX_OCTET_VALUE = 255,
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

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGCompressSettings {
	pub(super) btype: c_uint,
	pub(super) use_lz77: c_uint,
	pub(super) windowsize: c_uint,
	pub(super) minmatch: c_uint,
	pub(super) nicematch: c_uint,
	pub(super) lazymatching: c_uint,
	pub(super) custom_zlib: Option<
		unsafe extern "C-unwind" fn(
			arg1: *mut *mut c_uchar,
			arg2: *mut usize,
			arg3: *const c_uchar,
			arg4: usize,
			arg5: *const LodePNGCompressSettings,
		) -> c_uint,
	>,
	pub(super) custom_deflate: Option<
		unsafe extern "C-unwind" fn(
			arg1: *mut *mut c_uchar,
			arg2: *mut usize,
			arg3: *const c_uchar,
			arg4: usize,
			arg5: *const LodePNGCompressSettings,
		) -> c_uint,
	>,
	pub(super) custom_context: *const c_void,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGDecoderSettings {
	pub(super) zlibsettings: LodePNGDecompressSettings,
	pub(super) ignore_crc: c_uint,
	pub(super) ignore_critical: c_uint,
	pub(super) ignore_end: c_uint,
	pub(super) color_convert: c_uint,
	pub(super) read_text_chunks: c_uint,
	pub(super) remember_unknown_chunks: c_uint,
	pub(super) max_text_size: usize,
	pub(super) max_icc_size: usize,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGDecompressSettings {
	pub(super) ignore_adler32: c_uint,
	pub(super) ignore_nlen: c_uint,
	pub(super) max_output_size: usize,
	pub(super) custom_zlib: Option<
		unsafe extern "C-unwind" fn(
			arg1: *mut *mut c_uchar,
			arg2: *mut usize,
			arg3: *const c_uchar,
			arg4: usize,
			arg5: *const LodePNGDecompressSettings,
		) -> c_uint,
	>,
	pub(super) custom_inflate: Option<
		unsafe extern "C-unwind" fn(
			arg1: *mut *mut c_uchar,
			arg2: *mut usize,
			arg3: *const c_uchar,
			arg4: usize,
			arg5: *const LodePNGDecompressSettings,
		) -> c_uint,
	>,
	pub(super) custom_context: *const c_void,
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGEncoderSettings {
	pub(super) zlibsettings: LodePNGCompressSettings,
	pub(super) auto_convert: c_uint,
	pub(super) filter_palette_zero: c_uint,
	pub(super) filter_strategy: LodePNGFilterStrategy,
	pub(super) predefined_filters: *const c_uchar,
	pub(super) force_palette: c_uint,
	pub(super) add_id: c_uint,
	pub(super) text_compression: c_uint,
}

#[repr(u32)]
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub(super) enum LodePNGFilterStrategy {
	LFS_ZERO = 0,
	LFS_ONE = 1,
	LFS_TWO = 2,
	LFS_THREE = 3,
	LFS_FOUR = 4,
	LFS_MINSUM = 5,
	LFS_ENTROPY = 6,
	LFS_BRUTE_FORCE = 7,
	// LFS_PREDEFINED = 8,  // This strategy is redundant.
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGInfo {
	pub(super) compression_method: c_uint,
	pub(super) filter_method: c_uint,
	pub(super) interlace_method: c_uint,
	pub(super) color: LodePNGColorMode,
	pub(super) background_defined: c_uint,
	pub(super) background_r: c_uint,
	pub(super) background_g: c_uint,
	pub(super) background_b: c_uint,
	pub(super) text_num: usize,
	pub(super) text_keys: *mut *mut c_char,
	pub(super) text_strings: *mut *mut c_char,
	pub(super) itext_num: usize,
	pub(super) itext_keys: *mut *mut c_char,
	pub(super) itext_langtags: *mut *mut c_char,
	pub(super) itext_transkeys: *mut *mut c_char,
	pub(super) itext_strings: *mut *mut c_char,
	pub(super) time_defined: c_uint,
	pub(super) time: LodePNGTime,
	pub(super) phys_defined: c_uint,
	pub(super) phys_x: c_uint,
	pub(super) phys_y: c_uint,
	pub(super) phys_unit: c_uint,
	pub(super) gama_defined: c_uint,
	pub(super) gama_gamma: c_uint,
	pub(super) chrm_defined: c_uint,
	pub(super) chrm_white_x: c_uint,
	pub(super) chrm_white_y: c_uint,
	pub(super) chrm_red_x: c_uint,
	pub(super) chrm_red_y: c_uint,
	pub(super) chrm_green_x: c_uint,
	pub(super) chrm_green_y: c_uint,
	pub(super) chrm_blue_x: c_uint,
	pub(super) chrm_blue_y: c_uint,
	pub(super) srgb_defined: c_uint,
	pub(super) srgb_intent: c_uint,
	pub(super) iccp_defined: c_uint,
	pub(super) iccp_name: *mut c_char,
	pub(super) iccp_profile: *mut c_uchar,
	pub(super) iccp_profile_size: c_uint,
	pub(super) sbit_defined: c_uint,
	pub(super) sbit_r: c_uint,
	pub(super) sbit_g: c_uint,
	pub(super) sbit_b: c_uint,
	pub(super) sbit_a: c_uint,
	pub(super) unknown_chunks_data: [*mut c_uchar; 3usize],
	pub(super) unknown_chunks_size: [usize; 3usize],
}

#[repr(C)]
#[derive(Debug)]
pub(super) struct LodePNGState {
	pub(super) decoder: LodePNGDecoderSettings,
	pub(super) encoder: LodePNGEncoderSettings,
	pub(super) info_raw: LodePNGColorMode,
	pub(super) info_png: LodePNGInfo,
	pub(super) error: c_uint,
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
	pub(super) fn encode(&mut self, img: &DecodedImage) -> Option<EncodedImage<usize>> {
		// Safety: a non-zero response is an error.
		let mut out = EncodedImage::default();
		let res = unsafe {
			lodepng_encode(&mut out.buf, &mut out.size, img.buf, img.w, img.h, self)
		};

		// Return it if we got it.
		if 0 == res && ! out.is_empty() { Some(out) }
		else { None }
	}

	#[allow(unsafe_code)]
	/// # Set Up Encoder.
	///
	/// This configures and returns a new state for encoding purposes.
	pub(super) fn encoder(
		dec: &Self,
		strategy: LodePNGFilterStrategy,
		slow: bool
	) -> Option<Self> {
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
		enc.encoder.filter_strategy = strategy;
		enc.encoder.add_id = 0;
		enc.encoder.text_compression = 1;

		// For final compression, enable the custom zopfli deflater.
		if slow {
			enc.encoder.zlibsettings.windowsize = 32_768;
			enc.encoder.zlibsettings.custom_deflate = Some(custom_png_deflate);
		}
		else {
			enc.encoder.zlibsettings.windowsize = 8_192;
		}

		Some(enc)
	}

	#[allow(unsafe_code)]
	/// # Prepare Encoder for Encoding (a small image).
	///
	/// This updates an existing encoder to potentially further optimize a
	/// really small image.
	pub(super) fn prepare_encoder_small(&mut self, img: &DecodedImage) -> bool {
		// Safety: a non-zero response is an error.
		let mut stats = LodePNGColorStats::default();
		if 0 != unsafe {
			lodepng_compute_color_stats(&mut stats, img.buf, img.w, img.h, &self.info_raw)
		} { return false; }

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
		self.info_png.color.bitdepth = 8.min(stats.bits);

		// Rekey if necessary.
		if 0 == stats.alpha && 0 != stats.key {
			self.info_png.color.key_defined = 1;
			self.info_png.color.key_r = c_uint::from(stats.key_r) & 255;
			self.info_png.color.key_g = c_uint::from(stats.key_g) & 255;
			self.info_png.color.key_b = c_uint::from(stats.key_b) & 255;
		}
		else { self.info_png.color.key_defined = 0; }

		true
	}
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(super) struct LodePNGTime {
	pub(super) year: c_uint,
	pub(super) month: c_uint,
	pub(super) day: c_uint,
	pub(super) hour: c_uint,
	pub(super) minute: c_uint,
	pub(super) second: c_uint,
}



extern "C-unwind" {
	pub(super) fn custom_png_deflate(
		out: *mut *mut c_uchar,
		outsize: *mut usize,
		in_: *const c_uchar,
		insize: usize,
		settings: *const LodePNGCompressSettings,
	) -> c_uint;
}

extern "C-unwind" {
	pub(super) fn lodepng_color_mode_copy(
		dest: *mut LodePNGColorMode,
		source: *const LodePNGColorMode,
	) -> c_uint;
}

extern "C-unwind" {
	pub(super) fn lodepng_color_stats_init(stats: *mut LodePNGColorStats);
}

extern "C-unwind" {
	pub(super) fn lodepng_compute_color_stats(
		stats: *mut LodePNGColorStats,
		image: *const c_uchar,
		w: c_uint,
		h: c_uint,
		mode_in: *const LodePNGColorMode,
	) -> c_uint;
}

extern "C-unwind" {
	pub(super) fn lodepng_decode(
		out: *mut *mut c_uchar,
		w: *mut c_uint,
		h: *mut c_uint,
		state: *mut LodePNGState,
		in_: *const c_uchar,
		insize: usize,
	) -> c_uint;
}

extern "C-unwind" {
	pub(super) fn lodepng_encode(
		out: *mut *mut c_uchar,
		outsize: *mut usize,
		image: *const c_uchar,
		w: c_uint,
		h: c_uint,
		state: *mut LodePNGState,
	) -> c_uint;
}

extern "C-unwind" {
	pub(super) fn lodepng_state_cleanup(state: *mut LodePNGState);
}

extern "C-unwind" {
	pub(super) fn lodepng_state_init(state: *mut LodePNGState);
}



#[cfg(test)]
#[allow(deref_nullptr, non_snake_case, trivial_casts, unsafe_code)]
mod tests {
	use super::*;

	#[test]
	fn t_color_type_is_match() {
		for (p, t) in [
			("skel/assets/png/01.png", LodePNGColorType::LCT_RGB),
			("skel/assets/png/02.png", LodePNGColorType::LCT_RGBA),
			("skel/assets/png/04.png", LodePNGColorType::LCT_GREY),
			("skel/assets/png/small-bwa.png", LodePNGColorType::LCT_GREY_ALPHA),
		] {
			let raw = match std::fs::read(p) {
				Ok(x) => x,
				_ => panic!("Missing {}", p),
			};
			assert!(t.is_match(&raw));
		}

		// Let's test a negative to make sure we aren't doing something silly.
		let raw = std::fs::read("skel/assets/png/01.png").unwrap();
		assert!(! LodePNGColorType::LCT_GREY.is_match(&raw));
	}

	#[test]
	fn bindgen_test_layout_LodePNGState() {
		assert_eq!(
			::std::mem::size_of::<LodePNGState>(),
			544usize,
			concat!("Size of: ", stringify!(LodePNGState))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGState>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGState))
		);
		fn test_field_decoder() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGState>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).decoder) as usize - ptr as usize
				},
				0usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGState),
					"::",
					stringify!(decoder)
				)
			);
		}
		test_field_decoder();
		fn test_field_encoder() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGState>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).encoder) as usize - ptr as usize
				},
				80usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGState),
					"::",
					stringify!(encoder)
				)
			);
		}
		test_field_encoder();
		fn test_field_info_raw() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGState>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).info_raw) as usize - ptr as usize
				},
				168usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGState),
					"::",
					stringify!(info_raw)
				)
			);
		}
		test_field_info_raw();
		fn test_field_info_png() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGState>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).info_png) as usize - ptr as usize
				},
				208usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGState),
					"::",
					stringify!(info_png)
				)
			);
		}
		test_field_info_png();
		fn test_field_error() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGState>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).error) as usize - ptr as usize
				},
				536usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGState),
					"::",
					stringify!(error)
				)
			);
		}
		test_field_error();
	}

	#[test]
	fn bindgen_test_layout_LodePNGDecompressSettings() {
		assert_eq!(
			::std::mem::size_of::<LodePNGDecompressSettings>(),
			40usize,
			concat!("Size of: ", stringify!(LodePNGDecompressSettings))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGDecompressSettings>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGDecompressSettings))
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).ignore_adler32 as *const _
					as usize
			},
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(ignore_adler32)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).ignore_nlen as *const _ as usize
			},
			4usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(ignore_nlen)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).max_output_size as *const _
					as usize
			},
			8usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(max_output_size)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).custom_zlib as *const _ as usize
			},
			16usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(custom_zlib)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).custom_inflate as *const _
					as usize
			},
			24usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(custom_inflate)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecompressSettings>())).custom_context as *const _
					as usize
			},
			32usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecompressSettings),
				"::",
				stringify!(custom_context)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGCompressSettings() {
		assert_eq!(
			::std::mem::size_of::<LodePNGCompressSettings>(),
			48usize,
			concat!("Size of: ", stringify!(LodePNGCompressSettings))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGCompressSettings>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGCompressSettings))
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGCompressSettings>())).btype as *const _ as usize },
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(btype)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).use_lz77 as *const _ as usize
			},
			4usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(use_lz77)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).windowsize as *const _ as usize
			},
			8usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(windowsize)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).minmatch as *const _ as usize
			},
			12usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(minmatch)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).nicematch as *const _ as usize
			},
			16usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(nicematch)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).lazymatching as *const _ as usize
			},
			20usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(lazymatching)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).custom_zlib as *const _ as usize
			},
			24usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(custom_zlib)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).custom_deflate as *const _ as usize
			},
			32usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(custom_deflate)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGCompressSettings>())).custom_context as *const _ as usize
			},
			40usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGCompressSettings),
				"::",
				stringify!(custom_context)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGColorMode() {
		assert_eq!(
			::std::mem::size_of::<LodePNGColorMode>(),
			40usize,
			concat!("Size of: ", stringify!(LodePNGColorMode))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGColorMode>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGColorMode))
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).colortype as *const _ as usize },
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(colortype)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).bitdepth as *const _ as usize },
			4usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(bitdepth)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).palette as *const _ as usize },
			8usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(palette)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).palettesize as *const _ as usize },
			16usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(palettesize)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).key_defined as *const _ as usize },
			24usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(key_defined)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).key_r as *const _ as usize },
			28usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(key_r)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).key_g as *const _ as usize },
			32usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(key_g)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorMode>())).key_b as *const _ as usize },
			36usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorMode),
				"::",
				stringify!(key_b)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGTime() {
		assert_eq!(
			::std::mem::size_of::<LodePNGTime>(),
			24usize,
			concat!("Size of: ", stringify!(LodePNGTime))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGTime>(),
			4usize,
			concat!("Alignment of ", stringify!(LodePNGTime))
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).year as *const _ as usize },
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(year)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).month as *const _ as usize },
			4usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(month)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).day as *const _ as usize },
			8usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(day)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).hour as *const _ as usize },
			12usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(hour)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).minute as *const _ as usize },
			16usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(minute)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGTime>())).second as *const _ as usize },
			20usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGTime),
				"::",
				stringify!(second)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGInfo() {
		assert_eq!(
			::std::mem::size_of::<LodePNGInfo>(),
			328usize,
			concat!("Size of: ", stringify!(LodePNGInfo))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGInfo>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGInfo))
		);
		fn test_field_compression_method() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).compression_method) as usize - ptr as usize
				},
				0usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(compression_method)
				)
			);
		}
		test_field_compression_method();
		fn test_field_filter_method() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).filter_method) as usize - ptr as usize
				},
				4usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(filter_method)
				)
			);
		}
		test_field_filter_method();
		fn test_field_interlace_method() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).interlace_method) as usize - ptr as usize
				},
				8usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(interlace_method)
				)
			);
		}
		test_field_interlace_method();
		fn test_field_color() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).color) as usize - ptr as usize
				},
				16usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(color)
				)
			);
		}
		test_field_color();
		fn test_field_background_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).background_defined) as usize - ptr as usize
				},
				56usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(background_defined)
				)
			);
		}
		test_field_background_defined();
		fn test_field_background_r() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).background_r) as usize - ptr as usize
				},
				60usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(background_r)
				)
			);
		}
		test_field_background_r();
		fn test_field_background_g() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).background_g) as usize - ptr as usize
				},
				64usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(background_g)
				)
			);
		}
		test_field_background_g();
		fn test_field_background_b() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).background_b) as usize - ptr as usize
				},
				68usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(background_b)
				)
			);
		}
		test_field_background_b();
		fn test_field_text_num() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).text_num) as usize - ptr as usize
				},
				72usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(text_num)
				)
			);
		}
		test_field_text_num();
		fn test_field_text_keys() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).text_keys) as usize - ptr as usize
				},
				80usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(text_keys)
				)
			);
		}
		test_field_text_keys();
		fn test_field_text_strings() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).text_strings) as usize - ptr as usize
				},
				88usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(text_strings)
				)
			);
		}
		test_field_text_strings();
		fn test_field_itext_num() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).itext_num) as usize - ptr as usize
				},
				96usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(itext_num)
				)
			);
		}
		test_field_itext_num();
		fn test_field_itext_keys() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).itext_keys) as usize - ptr as usize
				},
				104usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(itext_keys)
				)
			);
		}
		test_field_itext_keys();
		fn test_field_itext_langtags() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).itext_langtags) as usize - ptr as usize
				},
				112usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(itext_langtags)
				)
			);
		}
		test_field_itext_langtags();
		fn test_field_itext_transkeys() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).itext_transkeys) as usize - ptr as usize
				},
				120usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(itext_transkeys)
				)
			);
		}
		test_field_itext_transkeys();
		fn test_field_itext_strings() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).itext_strings) as usize - ptr as usize
				},
				128usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(itext_strings)
				)
			);
		}
		test_field_itext_strings();
		fn test_field_time_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).time_defined) as usize - ptr as usize
				},
				136usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(time_defined)
				)
			);
		}
		test_field_time_defined();
		fn test_field_time() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).time) as usize - ptr as usize
				},
				140usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(time)
				)
			);
		}
		test_field_time();
		fn test_field_phys_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).phys_defined) as usize - ptr as usize
				},
				164usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(phys_defined)
				)
			);
		}
		test_field_phys_defined();
		fn test_field_phys_x() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).phys_x) as usize - ptr as usize
				},
				168usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(phys_x)
				)
			);
		}
		test_field_phys_x();
		fn test_field_phys_y() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).phys_y) as usize - ptr as usize
				},
				172usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(phys_y)
				)
			);
		}
		test_field_phys_y();
		fn test_field_phys_unit() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).phys_unit) as usize - ptr as usize
				},
				176usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(phys_unit)
				)
			);
		}
		test_field_phys_unit();
		fn test_field_gama_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).gama_defined) as usize - ptr as usize
				},
				180usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(gama_defined)
				)
			);
		}
		test_field_gama_defined();
		fn test_field_gama_gamma() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).gama_gamma) as usize - ptr as usize
				},
				184usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(gama_gamma)
				)
			);
		}
		test_field_gama_gamma();
		fn test_field_chrm_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_defined) as usize - ptr as usize
				},
				188usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_defined)
				)
			);
		}
		test_field_chrm_defined();
		fn test_field_chrm_white_x() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_white_x) as usize - ptr as usize
				},
				192usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_white_x)
				)
			);
		}
		test_field_chrm_white_x();
		fn test_field_chrm_white_y() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_white_y) as usize - ptr as usize
				},
				196usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_white_y)
				)
			);
		}
		test_field_chrm_white_y();
		fn test_field_chrm_red_x() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_red_x) as usize - ptr as usize
				},
				200usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_red_x)
				)
			);
		}
		test_field_chrm_red_x();
		fn test_field_chrm_red_y() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_red_y) as usize - ptr as usize
				},
				204usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_red_y)
				)
			);
		}
		test_field_chrm_red_y();
		fn test_field_chrm_green_x() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_green_x) as usize - ptr as usize
				},
				208usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_green_x)
				)
			);
		}
		test_field_chrm_green_x();
		fn test_field_chrm_green_y() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_green_y) as usize - ptr as usize
				},
				212usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_green_y)
				)
			);
		}
		test_field_chrm_green_y();
		fn test_field_chrm_blue_x() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_blue_x) as usize - ptr as usize
				},
				216usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_blue_x)
				)
			);
		}
		test_field_chrm_blue_x();
		fn test_field_chrm_blue_y() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).chrm_blue_y) as usize - ptr as usize
				},
				220usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(chrm_blue_y)
				)
			);
		}
		test_field_chrm_blue_y();
		fn test_field_srgb_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).srgb_defined) as usize - ptr as usize
				},
				224usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(srgb_defined)
				)
			);
		}
		test_field_srgb_defined();
		fn test_field_srgb_intent() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).srgb_intent) as usize - ptr as usize
				},
				228usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(srgb_intent)
				)
			);
		}
		test_field_srgb_intent();
		fn test_field_iccp_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).iccp_defined) as usize - ptr as usize
				},
				232usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(iccp_defined)
				)
			);
		}
		test_field_iccp_defined();
		fn test_field_iccp_name() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).iccp_name) as usize - ptr as usize
				},
				240usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(iccp_name)
				)
			);
		}
		test_field_iccp_name();
		fn test_field_iccp_profile() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).iccp_profile) as usize - ptr as usize
				},
				248usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(iccp_profile)
				)
			);
		}
		test_field_iccp_profile();
		fn test_field_iccp_profile_size() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).iccp_profile_size) as usize - ptr as usize
				},
				256usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(iccp_profile_size)
				)
			);
		}
		test_field_iccp_profile_size();
		fn test_field_sbit_defined() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).sbit_defined) as usize - ptr as usize
				},
				260usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(sbit_defined)
				)
			);
		}
		test_field_sbit_defined();
		fn test_field_sbit_r() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).sbit_r) as usize - ptr as usize
				},
				264usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(sbit_r)
				)
			);
		}
		test_field_sbit_r();
		fn test_field_sbit_g() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).sbit_g) as usize - ptr as usize
				},
				268usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(sbit_g)
				)
			);
		}
		test_field_sbit_g();
		fn test_field_sbit_b() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).sbit_b) as usize - ptr as usize
				},
				272usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(sbit_b)
				)
			);
		}
		test_field_sbit_b();
		fn test_field_sbit_a() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).sbit_a) as usize - ptr as usize
				},
				276usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(sbit_a)
				)
			);
		}
		test_field_sbit_a();
		fn test_field_unknown_chunks_data() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).unknown_chunks_data) as usize - ptr as usize
				},
				280usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(unknown_chunks_data)
				)
			);
		}
		test_field_unknown_chunks_data();
		fn test_field_unknown_chunks_size() {
			assert_eq!(
				unsafe {
					let uninit = ::std::mem::MaybeUninit::<LodePNGInfo>::uninit();
					let ptr = uninit.as_ptr();
					::std::ptr::addr_of!((*ptr).unknown_chunks_size) as usize - ptr as usize
				},
				304usize,
				concat!(
					"Offset of field: ",
					stringify!(LodePNGInfo),
					"::",
					stringify!(unknown_chunks_size)
				)
			);
		}
		test_field_unknown_chunks_size();
	}

	#[test]
	fn bindgen_test_layout_LodePNGDecoderSettings() {
		assert_eq!(
			::std::mem::size_of::<LodePNGDecoderSettings>(),
			80usize,
			concat!("Size of: ", stringify!(LodePNGDecoderSettings))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGDecoderSettings>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGDecoderSettings))
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).zlibsettings as *const _ as usize
			},
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(zlibsettings)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).ignore_crc as *const _ as usize
			},
			40usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(ignore_crc)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).ignore_critical as *const _ as usize
			},
			44usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(ignore_critical)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).ignore_end as *const _ as usize
			},
			48usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(ignore_end)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).color_convert as *const _ as usize
			},
			52usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(color_convert)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).read_text_chunks as *const _ as usize
			},
			56usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(read_text_chunks)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).remember_unknown_chunks as *const _
					as usize
			},
			60usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(remember_unknown_chunks)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).max_text_size as *const _ as usize
			},
			64usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(max_text_size)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGDecoderSettings>())).max_icc_size as *const _ as usize
			},
			72usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGDecoderSettings),
				"::",
				stringify!(max_icc_size)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGColorStats() {
		assert_eq!(
			::std::mem::size_of::<LodePNGColorStats>(),
			1072usize,
			concat!("Size of: ", stringify!(LodePNGColorStats))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGColorStats>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGColorStats))
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).colored as *const _ as usize },
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(colored)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).key as *const _ as usize },
			4usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(key)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).key_r as *const _ as usize },
			8usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(key_r)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).key_g as *const _ as usize },
			10usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(key_g)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).key_b as *const _ as usize },
			12usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(key_b)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).alpha as *const _ as usize },
			16usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(alpha)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).numcolors as *const _ as usize },
			20usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(numcolors)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).palette as *const _ as usize },
			24usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(palette)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).bits as *const _ as usize },
			1048usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(bits)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).numpixels as *const _ as usize },
			1056usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(numpixels)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGColorStats>())).allow_palette as *const _ as usize },
			1064usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(allow_palette)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGColorStats>())).allow_greyscale as *const _ as usize
			},
			1068usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGColorStats),
				"::",
				stringify!(allow_greyscale)
			)
		);
	}

	#[test]
	fn bindgen_test_layout_LodePNGEncoderSettings() {
		assert_eq!(
			::std::mem::size_of::<LodePNGEncoderSettings>(),
			88usize,
			concat!("Size of: ", stringify!(LodePNGEncoderSettings))
		);
		assert_eq!(
			::std::mem::align_of::<LodePNGEncoderSettings>(),
			8usize,
			concat!("Alignment of ", stringify!(LodePNGEncoderSettings))
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).zlibsettings as *const _ as usize
			},
			0usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(zlibsettings)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).auto_convert as *const _ as usize
			},
			48usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(auto_convert)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).filter_palette_zero as *const _
					as usize
			},
			52usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(filter_palette_zero)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).filter_strategy as *const _ as usize
			},
			56usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(filter_strategy)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).predefined_filters as *const _
					as usize
			},
			64usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(predefined_filters)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).force_palette as *const _ as usize
			},
			72usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(force_palette)
			)
		);
		assert_eq!(
			unsafe { &(*(::std::ptr::null::<LodePNGEncoderSettings>())).add_id as *const _ as usize },
			76usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(add_id)
			)
		);
		assert_eq!(
			unsafe {
				&(*(::std::ptr::null::<LodePNGEncoderSettings>())).text_compression as *const _ as usize
			},
			80usize,
			concat!(
				"Offset of field: ",
				stringify!(LodePNGEncoderSettings),
				"::",
				stringify!(text_compression)
			)
		);
	}
}
