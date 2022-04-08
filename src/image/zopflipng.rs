/*!
# Flaca: Zopflipng

This contains FFI bindings to libzopflipng, equivalent to:
```bash
zopflipng -m <input> <output>
```

The bindings themselves have been manually transcribed below, but the project's
`build.rs` includes the relevant `bindgen` notation — commented out — for
reference.
*/

use std::os::raw::{
	c_ulong,
	c_void,
};
use super::ImageKind;



#[allow(non_camel_case_types)]
mod raw {
	use std::os::raw::{
		c_char,
		c_int,
		c_uchar,
		c_uint,
		c_ulong,
	};

	type size_t = c_ulong;
	type ZopfliPNGFilterStrategy = c_uint;

	#[repr(C)]
	#[derive(Debug, Copy, Clone)]
	/// # (C) Zopflipng Options.
	pub(super) struct CZopfliPNGOptions {
		lossy_transparent: c_int,
		lossy_8bit: c_int,
		filter_strategies: *mut ZopfliPNGFilterStrategy,
		num_filter_strategies: c_int,
		auto_filter_strategy: c_int,
		keepchunks: *mut *mut c_char,
		num_keepchunks: c_int,
		use_zopfli: c_int,
		num_iterations: c_int,
		num_iterations_large: c_int,
		block_split_strategy: c_int,
	}

	impl Default for CZopfliPNGOptions {
		/// # Default.
		///
		/// These settings are equivalent to calling `zopflipng -m`.
		fn default() -> Self {
			Self {
				lossy_transparent: 0, // false
				lossy_8bit: 0, // false
				filter_strategies: std::ptr::null_mut(),
				num_filter_strategies: 0,
				auto_filter_strategy: 1, // true
				keepchunks: std::ptr::null_mut(),
				num_keepchunks: 0,
				use_zopfli: 1, // true
				num_iterations: 60,
				num_iterations_large: 20,
				block_split_strategy: 1,
			}
		}
	}

	#[cfg(test)]
	mod tests {
		use super::*;

		#[allow(trivial_casts)]
		#[allow(deref_nullptr)]
		#[test]
		fn test_bindgen() {
			assert_eq!(
				::std::mem::size_of::<CZopfliPNGOptions>(),
				56usize,
				concat!("Size of: ", stringify!(CZopfliPNGOptions))
			);
			assert_eq!(
				::std::mem::align_of::<CZopfliPNGOptions>(),
				8usize,
				concat!("Alignment of ", stringify!(CZopfliPNGOptions))
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).lossy_transparent as *const _ as usize
				},
				0usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(lossy_transparent)
				)
			);
			assert_eq!(
				unsafe { &(*(::std::ptr::null::<CZopfliPNGOptions>())).lossy_8bit as *const _ as usize },
				4usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(lossy_8bit)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).filter_strategies as *const _ as usize
				},
				8usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(filter_strategies)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).num_filter_strategies as *const _ as usize
				},
				16usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(num_filter_strategies)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).auto_filter_strategy as *const _ as usize
				},
				20usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(auto_filter_strategy)
				)
			);
			assert_eq!(
				unsafe { &(*(::std::ptr::null::<CZopfliPNGOptions>())).keepchunks as *const _ as usize },
				24usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(keepchunks)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).num_keepchunks as *const _ as usize
				},
				32usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(num_keepchunks)
				)
			);
			assert_eq!(
				unsafe { &(*(::std::ptr::null::<CZopfliPNGOptions>())).use_zopfli as *const _ as usize },
				36usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(use_zopfli)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).num_iterations as *const _ as usize
				},
				40usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(num_iterations)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).num_iterations_large as *const _ as usize
				},
				44usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(num_iterations_large)
				)
			);
			assert_eq!(
				unsafe {
					&(*(::std::ptr::null::<CZopfliPNGOptions>())).block_split_strategy as *const _ as usize
				},
				48usize,
				concat!(
					"Offset of field: ",
					stringify!(CZopfliPNGOptions),
					"::",
					stringify!(block_split_strategy)
				)
			);
		}
	}

	extern "C" {
		pub(super) fn CZopfliPNGOptimize(
			origpng: *const c_uchar,
			origpng_size: size_t,
			png_options: *const CZopfliPNGOptions,
			verbose: c_int,
			resultpng: *mut *mut c_uchar,
			resultpng_size: *mut size_t,
		) -> c_int;
	}
}



#[allow(unused_assignments)]
/// # Optimize!
pub(super) fn zopflipng_optimize(data: &mut Vec<u8>) {
	let data_size = data.len();
	let src_size = match c_ulong::try_from(data_size) {
		Ok(s) => s,
		Err(_) => return,
	};

	let mut out_ptr = std::ptr::null_mut();
	let mut out_size: c_ulong = 0;

	// Try to compress!
	let res: bool = 0 == unsafe {
		raw::CZopfliPNGOptimize(
			data.as_ptr(),
			src_size,
			&raw::CZopfliPNGOptions::default(),
			0, // false
			&mut out_ptr,
			&mut out_size,
		)
	};

	if 0 < out_size && ! out_ptr.is_null() {
		// Maybe replace the buffer with our new image!
		if res {
			if let Ok(size) = usize::try_from(out_size) {
				if size < data_size {
					let tmp = unsafe { std::slice::from_raw_parts(out_ptr, size) };
					if Some(ImageKind::Png) == ImageKind::parse(tmp) {
						data.truncate(size);
						data.copy_from_slice(tmp);
					}
				}
			}
		}

		// Manually free the C memory.
		unsafe { libc::free(out_ptr.cast::<c_void>()); }
		out_ptr = std::ptr::null_mut();
	}
}
