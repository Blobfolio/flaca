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

#[allow(unused_extern_crates)] // This fixes a linker issue.
extern crate link_cplusplus;



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



#[allow(unused_assignments)]
#[must_use]
/// # Optimize!
pub(super) fn zopflipng_optimize(src: &[u8]) -> Option<Vec<u8>> {
	let src_ptr = src.as_ptr();
	let src_size: u64 = u64::try_from(src.len()).ok()?;

	let mut out_ptr = std::ptr::null_mut();
	let mut out_size: u64 = 0;

	// Initialize the options equivalent to calling the binary with the `-m`
	// flag.
	let options = raw::CZopfliPNGOptions {
		lossy_transparent: i32::from(false),
		lossy_8bit: i32::from(false),
		filter_strategies: std::ptr::null_mut(),
		num_filter_strategies: 0,
		auto_filter_strategy: i32::from(true),
		keepchunks: std::ptr::null_mut(),
		num_keepchunks: 0,
		use_zopfli: i32::from(true),
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
			i32::from(false),
			&mut out_ptr,
			&mut out_size,
		)
	};

	let out: Vec<u8> =
		if out_ptr.is_null() || out_size == 0 { Vec::new() }
		else {
			unsafe {
				// Copy the data to a Rust vec.
				let tmp = std::slice::from_raw_parts(
					out_ptr,
					usize::try_from(out_size).unwrap_or_default(),
				).to_vec();

				// Manually free the C memory.
				libc::free(out_ptr.cast::<libc::c_void>());
				out_ptr = std::ptr::null_mut();
				out_size = 0;

				tmp
			}
		};

	// Done!
	if res == 0 { Some(out) }
	else { None }
}
