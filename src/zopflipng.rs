/*!
# Flaca: Zopflipng

This contains FFI bindings to zopflipng.
*/

#[allow(unused_extern_crates)] // This fixes a linker issue.
extern crate link_cplusplus;

use std::{
	mem::ManuallyDrop,
	os::raw::c_char,
};



#[allow(non_camel_case_types)]
mod raw {
	use std::os::raw::{
		c_char,
		c_int,
		c_uint,
		c_ulong,
	};

	pub(super) type size_t = c_ulong;
	pub(super) type ZopfliPNGFilterStrategy = c_uint;

	#[repr(C)]
	#[derive(Debug, Copy, Clone)]
	/// # (C) Zopflipng Options.
	pub(super) struct CZopfliPNGOptions {
		pub(super) lossy_transparent: c_int,
		pub(super) lossy_8bit: c_int,
		pub(super) filter_strategies: *mut ZopfliPNGFilterStrategy,
		pub(super) num_filter_strategies: c_int,
		pub(super) auto_filter_strategy: c_int,
		pub(super) keepchunks: *mut *mut c_char,
		pub(super) num_keepchunks: c_int,
		pub(super) use_zopfli: c_int,
		pub(super) num_iterations: c_int,
		pub(super) num_iterations_large: c_int,
		pub(super) block_split_strategy: c_int,
	}

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

	extern "C" {
		pub(super) fn CZopfliPNGOptimize(
			origpng: *const ::std::os::raw::c_uchar,
			origpng_size: size_t,
			png_options: *const CZopfliPNGOptions,
			verbose: c_int,
			resultpng: *mut *mut ::std::os::raw::c_uchar,
			resultpng_size: *mut size_t,
		) -> c_int;
	}
}



#[must_use]
/// # Optimize!
pub(super) fn zopflipng_optimize(src: &[u8]) -> Option<Vec<u8>> {
	let src_ptr = src.as_ptr();
	let src_size: u64 = u64::try_from(src.len()).ok()?;

	// Start an output buffer with a slightly larger capacity than the source,
	// just in case zopfli makes things worse, then immediately dissolve it so
	// we can play with the raw pointer.
	let out_cap: usize = src.len().saturating_add(1_048_576);
	let out: Vec<u8> = Vec::with_capacity(out_cap);
	let mut out = ManuallyDrop::new(out);
	let mut out_ptr = out.as_mut_ptr();
	let mut out_size: u64 = 0;

	// Initialize the options equivalent to calling the binary with the `-m`
	// flag. We aren't using either strategies or keepchunks, but the C struct
	// wants mutable pointers to them, so we'll register them explicitly just
	// in case.
	let mut strat: Box<[raw::ZopfliPNGFilterStrategy]> = Box::from([]);
	let mut keep: Box<[*mut c_char]> = Box::from([]);
	let options = raw::CZopfliPNGOptions {
		lossy_transparent: false as _,
		lossy_8bit: false as _,
		filter_strategies: strat.as_mut_ptr(),
		num_filter_strategies: 0,
		auto_filter_strategy: true as _,
		keepchunks: keep.as_mut_ptr(),
		num_keepchunks: 0,
		use_zopfli: true as _,
		num_iterations: 60,
		num_iterations_large: 20,
		block_split_strategy: 1,
	};

	// Try to compress!
	let res = unsafe {
		raw::CZopfliPNGOptimize(
			src_ptr,
			src_size,
			&options,
			false as _,
			&mut out_ptr,
			&mut out_size,
		)
	};

	// Rebuild the vec so the memory can be dropped, etc.
	let rebuilt = unsafe {
		Vec::from_raw_parts(
			out_ptr,
			usize::try_from(out_size).unwrap_or_default(),
			out_cap,
		)
	};

	// It worked!
	if res == 0 && 0 < out_size && out_size < src_size {
		Some(rebuilt)
	}
	else { None }
}
