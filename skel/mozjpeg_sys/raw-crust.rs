#![allow(
	dead_code,
	mutable_transmutes,
	non_camel_case_types,
	non_snake_case,
	non_upper_case_globals,
	unused_assignments,
	unused_mut
)]
#![register_tool(c2rust)]
#![feature(
	const_raw_ptr_to_usize_cast,
	const_transmute,
	extern_types,
	main,
	register_tool
)]
pub mod jconfigint_h {
	pub const BUILD: [libc::c_char; 9] =
		unsafe { *::std::mem::transmute::<&[u8; 9], &[libc::c_char; 9]>(b"20191212\x00") };
	/* Compiler's inline keyword */
	/* How to obtain function inlining. */
	/* Define to the full name of this package. */

	pub const PACKAGE_NAME: [libc::c_char; 8] =
		unsafe { *::std::mem::transmute::<&[u8; 8], &[libc::c_char; 8]>(b"mozjpeg\x00") };
	/* Version number of package */

	pub const VERSION: [libc::c_char; 6] =
		unsafe { *::std::mem::transmute::<&[u8; 6], &[libc::c_char; 6]>(b"4.0.0\x00") };
}
pub mod jversion_h {
	pub const JVERSION: [libc::c_char; 16] =
		unsafe { *::std::mem::transmute::<&[u8; 16], &[libc::c_char; 16]>(b"6b  27-Mar-1998\x00") };
	/*
	 * NOTE: It is our convention to place the authors in the following order:
	 * - libjpeg-turbo authors (2009-) in descending order of the date of their
	 *   most recent contribution to the project, then in ascending order of the
	 *   date of their first contribution to the project
	 * - Upstream authors in descending order of the date of the first inclusion of
	 *   their code
	 */

	pub const JCOPYRIGHT: [libc::c_char; 533] = unsafe {
		*::std::mem::transmute::<&[u8; 533],
									 &[libc::c_char; 533]>(b"Copyright (C) 2009-2018 D. R. Commander\nCopyright (C) 2011-2016 Siarhei Siamashka\nCopyright (C) 2015-2016, 2018 Matthieu Darbois\nCopyright (C) 2015 Intel Corporation\nCopyright (C) 2015 Google, Inc.\nCopyright (C) 2014 Mozilla Corporation\nCopyright (C) 2013-2014 MIPS Technologies, Inc.\nCopyright (C) 2013 Linaro Limited\nCopyright (C) 2009-2011 Nokia Corporation and/or its subsidiary(-ies)\nCopyright (C) 2009 Pierre Ossman for Cendio AB\nCopyright (C) 1999-2006 MIYASAKA Masaru\nCopyright (C) 1991-2016 Thomas G. Lane, Guido Vollbeding\x00")
	};
}
pub mod jconfig_h {
	pub const JPEG_LIB_VERSION: libc::c_int = 62 as libc::c_int;
}
pub mod cdjpeg_h {
	extern "C" {
		#[no_mangle]
		pub fn read_scan_script(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			filename: *mut libc::c_char,
		) -> crate::jmorecfg_h::boolean;

		#[no_mangle]
		pub fn keymatch(
			arg: *mut libc::c_char,
			keyword: *const libc::c_char,
			minchars: libc::c_int,
		) -> crate::jmorecfg_h::boolean;

		#[no_mangle]
		pub fn read_stdin() -> *mut crate::stdlib::FILE;

		#[no_mangle]
		pub fn write_stdout() -> *mut crate::stdlib::FILE;
	}
	pub const READ_BINARY: [libc::c_char; 3] =
		unsafe { *::std::mem::transmute::<&[u8; 3], &[libc::c_char; 3]>(b"rb\x00") };

	pub const WRITE_BINARY: [libc::c_char; 3] =
		unsafe { *::std::mem::transmute::<&[u8; 3], &[libc::c_char; 3]>(b"wb\x00") };
	/* define exit() codes if not provided */

	pub const EXIT_WARNING: libc::c_int = 2 as libc::c_int;
}
pub mod transupp_h {
	extern "C" {
		#[no_mangle]
		pub fn jtransform_parse_crop_spec(
			info: *mut crate::transupp_h::jpeg_transform_info,
			spec: *const libc::c_char,
		) -> crate::jmorecfg_h::boolean;
		/* Request any required workspace */
		#[no_mangle]
		pub fn jtransform_request_workspace(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			info: *mut crate::transupp_h::jpeg_transform_info,
		) -> crate::jmorecfg_h::boolean;
		/* Adjust output image parameters */
		#[no_mangle]
		pub fn jtransform_adjust_parameters(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			dstinfo: crate::jpeglib_h::j_compress_ptr,
			src_coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr,
			info: *mut crate::transupp_h::jpeg_transform_info,
		) -> *mut crate::jpeglib_h::jvirt_barray_ptr;
		/* Execute the actual transformation, if any */
		#[no_mangle]
		pub fn jtransform_execute_transform(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			dstinfo: crate::jpeglib_h::j_compress_ptr,
			src_coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr,
			info: *mut crate::transupp_h::jpeg_transform_info,
		);
		/* recommended default */
		/* Setup decompression object to save desired markers in memory */
		#[no_mangle]
		pub fn jcopy_markers_setup(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			option: crate::transupp_h::JCOPY_OPTION,
		);
		/* Copy markers saved in the given source object to the destination object */
		#[no_mangle]
		pub fn jcopy_markers_execute(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			dstinfo: crate::jpeglib_h::j_compress_ptr,
			option: crate::transupp_h::JCOPY_OPTION,
		);
	}
	pub type JXFORM_CODE = libc::c_uint;

	pub type JCROP_CODE = libc::c_uint;

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_transform_info {
		pub transform: crate::transupp_h::JXFORM_CODE,
		pub perfect: crate::jmorecfg_h::boolean,
		pub trim: crate::jmorecfg_h::boolean,
		pub force_grayscale: crate::jmorecfg_h::boolean,
		pub crop: crate::jmorecfg_h::boolean,
		pub slow_hflip: crate::jmorecfg_h::boolean,
		pub crop_width: crate::jmorecfg_h::JDIMENSION,
		pub crop_width_set: crate::transupp_h::JCROP_CODE,
		pub crop_height: crate::jmorecfg_h::JDIMENSION,
		pub crop_height_set: crate::transupp_h::JCROP_CODE,
		pub crop_xoffset: crate::jmorecfg_h::JDIMENSION,
		pub crop_xoffset_set: crate::transupp_h::JCROP_CODE,
		pub crop_yoffset: crate::jmorecfg_h::JDIMENSION,
		pub crop_yoffset_set: crate::transupp_h::JCROP_CODE,
		pub num_components: libc::c_int,
		pub workspace_coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr,
		pub output_width: crate::jmorecfg_h::JDIMENSION,
		pub output_height: crate::jmorecfg_h::JDIMENSION,
		pub x_crop_offset: crate::jmorecfg_h::JDIMENSION,
		pub y_crop_offset: crate::jmorecfg_h::JDIMENSION,
		pub iMCU_sample_width: libc::c_int,
		pub iMCU_sample_height: libc::c_int,
	}

	pub type JCOPY_OPTION = libc::c_uint;

	pub const JXFORM_ROT_270: crate::transupp_h::JXFORM_CODE = 7;

	pub const JXFORM_ROT_180: crate::transupp_h::JXFORM_CODE = 6;

	pub const JXFORM_ROT_90: crate::transupp_h::JXFORM_CODE = 5;

	pub const JXFORM_TRANSVERSE: crate::transupp_h::JXFORM_CODE = 4;

	pub const JXFORM_TRANSPOSE: crate::transupp_h::JXFORM_CODE = 3;

	pub const JXFORM_FLIP_V: crate::transupp_h::JXFORM_CODE = 2;

	pub const JXFORM_FLIP_H: crate::transupp_h::JXFORM_CODE = 1;

	pub const JXFORM_NONE: crate::transupp_h::JXFORM_CODE = 0;

	pub const JCROP_FORCE: crate::transupp_h::JCROP_CODE = 3;

	pub const JCROP_NEG: crate::transupp_h::JCROP_CODE = 2;

	pub const JCROP_POS: crate::transupp_h::JCROP_CODE = 1;

	pub const JCROP_UNSET: crate::transupp_h::JCROP_CODE = 0;

	pub const JCOPYOPT_ALL_EXCEPT_ICC: crate::transupp_h::JCOPY_OPTION = 3;

	pub const JCOPYOPT_ALL: crate::transupp_h::JCOPY_OPTION = 2;

	pub const JCOPYOPT_COMMENTS: crate::transupp_h::JCOPY_OPTION = 1;

	pub const JCOPYOPT_NONE: crate::transupp_h::JCOPY_OPTION = 0;
	/* jtransform_execute_transform used to be called
	 * jtransform_execute_transformation, but some compilers complain about
	 * routine names that long.  This macro is here to avoid breaking any
	 * old source code that uses the original name...
	 */

	pub const jtransform_execute_transformation: unsafe extern "C" fn(
		_: crate::jpeglib_h::j_decompress_ptr,
		_: crate::jpeglib_h::j_compress_ptr,
		_: *mut crate::jpeglib_h::jvirt_barray_ptr,
		_: *mut crate::transupp_h::jpeg_transform_info,
	) -> () = crate::transupp_h::jtransform_execute_transform;

	pub const JCOPYOPT_DEFAULT: libc::c_int = crate::transupp_h::JCOPYOPT_COMMENTS as libc::c_int;
}
pub mod jpeglib_h {
	extern "C" {
		pub type jvirt_barray_control;

		pub type jvirt_sarray_control;

		pub type jpeg_entropy_encoder;

		pub type jpeg_forward_dct;

		pub type jpeg_downsampler;

		pub type jpeg_color_converter;

		pub type jpeg_marker_writer;

		pub type jpeg_c_coef_controller;

		pub type jpeg_c_prep_controller;

		pub type jpeg_c_main_controller;

		pub type jpeg_comp_master;

		pub type jpeg_color_quantizer;

		pub type jpeg_color_deconverter;

		pub type jpeg_upsampler;

		pub type jpeg_inverse_dct;

		pub type jpeg_entropy_decoder;

		pub type jpeg_marker_reader;

		pub type jpeg_input_controller;

		pub type jpeg_d_post_controller;

		pub type jpeg_d_coef_controller;

		pub type jpeg_d_main_controller;

		pub type jpeg_decomp_master;
		/* Originally, this macro was used as a way of defining function prototypes
		 * for both modern compilers as well as older compilers that did not support
		 * prototype parameters.  libjpeg-turbo has never supported these older,
		 * non-ANSI compilers, but the macro is still included because there is some
		 * software out there that uses it.
		 */
		/* Default error-management setup */
		#[no_mangle]
		pub fn jpeg_std_error(
			err: *mut crate::jpeglib_h::jpeg_error_mgr,
		) -> *mut crate::jpeglib_h::jpeg_error_mgr;
		/* Initialization of JPEG compression objects.
		 * jpeg_create_compress() and jpeg_create_decompress() are the exported
		 * names that applications should call.  These expand to calls on
		 * jpeg_CreateCompress and jpeg_CreateDecompress with additional information
		 * passed for version mismatch checking.
		 * NB: you must set up the error-manager BEFORE calling jpeg_create_xxx.
		 */
		#[no_mangle]
		pub fn jpeg_CreateCompress(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			version: libc::c_int,
			structsize: crate::stddef_h::size_t,
		);

		#[no_mangle]
		pub fn jpeg_CreateDecompress(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
			version: libc::c_int,
			structsize: crate::stddef_h::size_t,
		);
		/* Destruction of JPEG compression objects */
		#[no_mangle]
		pub fn jpeg_destroy_compress(cinfo: crate::jpeglib_h::j_compress_ptr);

		#[no_mangle]
		pub fn jpeg_destroy_decompress(cinfo: crate::jpeglib_h::j_decompress_ptr);
		/* Standard data source and destination managers: stdio streams. */
		/* Caller is responsible for opening the file before and closing after. */
		#[no_mangle]
		pub fn jpeg_stdio_dest(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			outfile: *mut crate::stdlib::FILE,
		);

		#[no_mangle]
		pub fn jpeg_stdio_src(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
			infile: *mut crate::stdlib::FILE,
		);
		/* Data source and destination managers: memory buffers. */
		#[no_mangle]
		pub fn jpeg_mem_dest(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			outbuffer: *mut *mut libc::c_uchar,
			outsize: *mut libc::c_ulong,
		);

		#[no_mangle]
		pub fn jpeg_mem_src(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
			inbuffer: *const libc::c_uchar,
			insize: libc::c_ulong,
		);

		#[no_mangle]
		pub fn jpeg_simple_progression(cinfo: crate::jpeglib_h::j_compress_ptr);

		#[no_mangle]
		pub fn jpeg_finish_compress(cinfo: crate::jpeglib_h::j_compress_ptr);
		/* Write ICC profile.  See libjpeg.txt for usage information. */
		#[no_mangle]
		pub fn jpeg_write_icc_profile(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			icc_data_ptr: *const crate::jmorecfg_h::JOCTET,
			icc_data_len: libc::c_uint,
		);
		/* Decompression startup: read start of JPEG datastream to see what's there */
		#[no_mangle]
		pub fn jpeg_read_header(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
			require_image: crate::jmorecfg_h::boolean,
		) -> libc::c_int;

		#[no_mangle]
		pub fn jpeg_finish_decompress(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
		) -> crate::jmorecfg_h::boolean;
		/* Read or write raw DCT coefficients --- useful for lossless transcoding. */
		#[no_mangle]
		pub fn jpeg_read_coefficients(
			cinfo: crate::jpeglib_h::j_decompress_ptr,
		) -> *mut crate::jpeglib_h::jvirt_barray_ptr;

		#[no_mangle]
		pub fn jpeg_write_coefficients(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr,
		);

		#[no_mangle]
		pub fn jpeg_copy_critical_parameters(
			srcinfo: crate::jpeglib_h::j_decompress_ptr,
			dstinfo: crate::jpeglib_h::j_compress_ptr,
		);

		#[no_mangle]
		pub fn jpeg_c_set_bool_param(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			param: crate::jpeglib_h::J_BOOLEAN_PARAM,
			value: crate::jmorecfg_h::boolean,
		);

		#[no_mangle]
		pub fn jpeg_c_int_param_supported(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			param: crate::jpeglib_h::J_INT_PARAM,
		) -> crate::jmorecfg_h::boolean;

		#[no_mangle]
		pub fn jpeg_c_set_int_param(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			param: crate::jpeglib_h::J_INT_PARAM,
			value: libc::c_int,
		);

		#[no_mangle]
		pub fn jpeg_c_get_int_param(
			cinfo: crate::jpeglib_h::j_compress_ptr,
			param: crate::jpeglib_h::J_INT_PARAM,
		) -> libc::c_int;
	}
	pub type C2RustUnnamed_0 = libc::c_uint;

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub union C2RustUnnamed_1 {
		pub i: [libc::c_int; 8],
		pub s: [libc::c_char; 80],
	}
	/*
	 * jpeglib.h
	 *
	 * This file was part of the Independent JPEG Group's software:
	 * Copyright (C) 1991-1998, Thomas G. Lane.
	 * Modified 2002-2009 by Guido Vollbeding.
	 * libjpeg-turbo Modifications:
	 * Copyright (C) 2009-2011, 2013-2014, 2016-2017, D. R. Commander.
	 * Copyright (C) 2015, Google, Inc.
	 * mozjpeg Modifications:
	 * Copyright (C) 2014, Mozilla Corporation.
	 * For conditions of distribution and use, see the accompanying README.ijg
	 * file.
	 *
	 * This file defines the application interface for the JPEG library.
	 * Most applications using the library need only include this file,
	 * and perhaps jerror.h if they want to know the exact error codes.
	 */
	/*
	 * First we include the configuration files that record how this
	 * installation of the JPEG library is set up.  jconfig.h can be
	 * generated automatically for many systems.  jmorecfg.h contains
	 * manual configuration options that most people need not worry about.
	 */
	/* in case jinclude.h already did */
	/* Various constants determining the sizes of things.
	 * All of these are specified by the JPEG standard, so don't change them
	 * if you want to be compatible.
	 */
	/* The basic DCT block is 8x8 samples */
	/* DCTSIZE squared; # of elements in a block */
	/* Quantization tables are numbered 0..3 */
	/* Huffman tables are numbered 0..3 */
	/* Arith-coding tables are numbered 0..15 */
	/* JPEG limit on # of components in one scan */
	/* JPEG limit on sampling factors */
	/* Unfortunately, some bozo at Adobe saw no reason to be bound by the standard;
	 * the PostScript DCT filter can emit files with many more than 10 blocks/MCU.
	 * If you happen to run across such a file, you can up D_MAX_BLOCKS_IN_MCU
	 * to handle it.  We even let you do this from the jconfig.h file.  However,
	 * we strongly discourage changing C_MAX_BLOCKS_IN_MCU; just because Adobe
	 * sometimes emits noncompliant files doesn't mean you should too.
	 */
	/* compressor's limit on blocks per MCU */
	/* decompressor's limit on blocks per MCU */
	/* Data structures for images (arrays of samples and of DCT coefficients).
	 */

	pub type JSAMPROW = *mut crate::jmorecfg_h::JSAMPLE;
	/* ptr to one image row of pixel samples. */

	pub type JSAMPARRAY = *mut crate::jpeglib_h::JSAMPROW;
	/* a 3-D sample array: top index is color */

	pub type JBLOCK = [crate::jmorecfg_h::JCOEF; 64];
	/* one block of coefficients */

	pub type JBLOCKROW = *mut crate::jpeglib_h::JBLOCK;
	/* pointer to one row of coefficient blocks */

	pub type JBLOCKARRAY = *mut crate::jpeglib_h::JBLOCKROW;
	/* useful in a couple of places */
	/* Types for JPEG compression parameters and working tables. */
	/* DCT coefficient quantization tables. */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct JQUANT_TBL {
		pub quantval: [crate::jmorecfg_h::UINT16; 64],
		pub sent_table: crate::jmorecfg_h::boolean,
	}
	/* Huffman coding tables. */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct JHUFF_TBL {
		pub bits: [crate::jmorecfg_h::UINT8; 17],
		pub huffval: [crate::jmorecfg_h::UINT8; 256],
		pub sent_table: crate::jmorecfg_h::boolean,
	}
	/* Basic info about one component (color channel). */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_component_info {
		pub component_id: libc::c_int,
		pub component_index: libc::c_int,
		pub h_samp_factor: libc::c_int,
		pub v_samp_factor: libc::c_int,
		pub quant_tbl_no: libc::c_int,
		pub dc_tbl_no: libc::c_int,
		pub ac_tbl_no: libc::c_int,
		pub width_in_blocks: crate::jmorecfg_h::JDIMENSION,
		pub height_in_blocks: crate::jmorecfg_h::JDIMENSION,
		pub DCT_scaled_size: libc::c_int,
		pub downsampled_width: crate::jmorecfg_h::JDIMENSION,
		pub downsampled_height: crate::jmorecfg_h::JDIMENSION,
		pub component_needed: crate::jmorecfg_h::boolean,
		pub MCU_width: libc::c_int,
		pub MCU_height: libc::c_int,
		pub MCU_blocks: libc::c_int,
		pub MCU_sample_width: libc::c_int,
		pub last_col_width: libc::c_int,
		pub last_row_height: libc::c_int,
		pub quant_table: *mut crate::jpeglib_h::JQUANT_TBL,
		pub dct_table: *mut libc::c_void,
	}
	/* The script for encoding a multiple-scan file is an array of these: */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_scan_info {
		pub comps_in_scan: libc::c_int,
		pub component_index: [libc::c_int; 4],
		pub Ss: libc::c_int,
		pub Se: libc::c_int,
		pub Ah: libc::c_int,
		pub Al: libc::c_int,
	}
	/* The decompressor can save APPn and COM markers in a list of these: */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_marker_struct {
		pub next: crate::jpeglib_h::jpeg_saved_marker_ptr,
		pub marker: crate::jmorecfg_h::UINT8,
		pub original_length: libc::c_uint,
		pub data_length: libc::c_uint,
		pub data: *mut crate::jmorecfg_h::JOCTET,
	}

	pub type jpeg_saved_marker_ptr = *mut crate::jpeglib_h::jpeg_marker_struct;
	/* the data contained in the marker */
	/* the marker length word is not counted in data_length or original_length */
	/* Known color spaces. */

	pub type J_COLOR_SPACE = libc::c_uint;
	/* DCT/IDCT algorithm options. */

	pub type J_DCT_METHOD = libc::c_uint;
	/* may be overridden in jconfig.h */
	/* may be overridden in jconfig.h */
	/* Dithering options for decompression. */

	pub type J_DITHER_MODE = libc::c_uint;
	/* These 32-bit GUIDs and the corresponding jpeg_*_get_*_param()/
	 * jpeg_*_set_*_param() functions allow for extending the libjpeg API without
	 * breaking backward ABI compatibility.  The actual parameters are stored in
	 * the opaque jpeg_comp_master and jpeg_decomp_master structs.
	 */
	/* Boolean extension parameters */

	pub type J_BOOLEAN_PARAM = libc::c_uint;
	/* Integer parameters */

	pub type J_INT_PARAM = libc::c_uint;
	/* Common fields between JPEG compression and decompression master structs. */
	/* Error handler module */
	/* Memory manager module */
	/* Progress monitor, or NULL if none */
	/* Available for use by application */
	/* So common code can tell which is which */
	/* For checking call sequence validity */
	/* Routines that are to be used by both halves of the library are declared
	 * to receive a pointer to this structure.  There are no actual instances of
	 * jpeg_common_struct, only of jpeg_compress_struct and jpeg_decompress_struct.
	 */

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_common_struct {
		pub err: *mut crate::jpeglib_h::jpeg_error_mgr,
		pub mem: *mut crate::jpeglib_h::jpeg_memory_mgr,
		pub progress: *mut crate::jpeglib_h::jpeg_progress_mgr,
		pub client_data: *mut libc::c_void,
		pub is_decompressor: crate::jmorecfg_h::boolean,
		pub global_state: libc::c_int,
	}

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_progress_mgr {
		pub progress_monitor: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub pass_counter: libc::c_long,
		pub pass_limit: libc::c_long,
		pub completed_passes: libc::c_int,
		pub total_passes: libc::c_int,
	}
	/* Fields common to both master struct types */
	/* Additional fields follow in an actual jpeg_compress_struct or
	 * jpeg_decompress_struct.  All three structs must agree on these
	 * initial fields!  (This would be a lot cleaner in C++.)
	 */

	pub type j_common_ptr = *mut crate::jpeglib_h::jpeg_common_struct;

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_memory_mgr {
		pub alloc_small: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::stddef_h::size_t,
			) -> *mut libc::c_void,
		>,
		pub alloc_large: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::stddef_h::size_t,
			) -> *mut libc::c_void,
		>,
		pub alloc_sarray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
			) -> crate::jpeglib_h::JSAMPARRAY,
		>,
		pub alloc_barray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
			) -> crate::jpeglib_h::JBLOCKARRAY,
		>,
		pub request_virt_sarray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::jmorecfg_h::boolean,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
			) -> crate::jpeglib_h::jvirt_sarray_ptr,
		>,
		pub request_virt_barray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: libc::c_int,
				_: crate::jmorecfg_h::boolean,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
			) -> crate::jpeglib_h::jvirt_barray_ptr,
		>,
		pub realize_virt_arrays:
			Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub access_virt_sarray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: crate::jpeglib_h::jvirt_sarray_ptr,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::boolean,
			) -> crate::jpeglib_h::JSAMPARRAY,
		>,
		pub access_virt_barray: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_common_ptr,
				_: crate::jpeglib_h::jvirt_barray_ptr,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::JDIMENSION,
				_: crate::jmorecfg_h::boolean,
			) -> crate::jpeglib_h::JBLOCKARRAY,
		>,
		pub free_pool:
			Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr, _: libc::c_int) -> ()>,
		pub self_destruct: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub max_memory_to_use: libc::c_long,
		pub max_alloc_chunk: libc::c_long,
	}
	/* Master record for a compression instance */
	/* Fields shared with jpeg_decompress_struct */
	/* Destination for compressed data */
	/* Description of source image --- these fields must be filled in by
	 * outer application before starting compression.  in_color_space must
	 * be correct before you can even call jpeg_set_defaults().
	 */
	/* input image width */
	/* input image height */
	/* # of color components in input image */
	/* colorspace of input image */
	/* image gamma of input image */
	/* Compression parameters --- these fields must be set before calling
	 * jpeg_start_compress().  We recommend calling jpeg_set_defaults() to
	 * initialize everything to reasonable defaults, then changing anything
	 * the application specifically wants to change.  That way you won't get
	 * burnt when new parameters are added.  Also note that there are several
	 * helper routines to simplify changing parameters.
	 */
	/* bits of precision in image data */
	/* # of color components in JPEG image */
	/* colorspace of JPEG image */
	/* comp_info[i] describes component that appears i'th in SOF */
	/* ptrs to coefficient quantization tables, or NULL if not defined,
	 * and corresponding scale factors (percentage, initialized 100).
	 */
	/* ptrs to Huffman coding tables, or NULL if not defined */
	/* L values for DC arith-coding tables */
	/* U values for DC arith-coding tables */
	/* Kx values for AC arith-coding tables */
	/* # of entries in scan_info array */
	/* script for multi-scan file, or NULL */
	/* The default value of scan_info is NULL, which causes a single-scan
	 * sequential JPEG file to be emitted.  To create a multi-scan file,
	 * set num_scans and scan_info to point to an array of scan definitions.
	 */
	/* TRUE=caller supplies downsampled data */
	/* TRUE=arithmetic coding, FALSE=Huffman */
	/* TRUE=optimize entropy encoding parms */
	/* TRUE=first samples are cosited */
	/* 1..100, or 0 for no input smoothing */
	/* DCT algorithm selector */
	/* The restart interval can be specified in absolute MCUs by setting
	 * restart_interval, or in MCU rows by setting restart_in_rows
	 * (in which case the correct restart_interval will be figured
	 * for each scan).
	 */
	/* MCUs per restart, or 0 for no restart */
	/* if > 0, MCU rows per restart interval */
	/* Parameters controlling emission of special markers. */
	/* should a JFIF marker be written? */
	/* What to write for the JFIF version number */
	/* These three values are not used by the JPEG code, merely copied */
	/* into the JFIF APP0 marker.  density_unit can be 0 for unknown, */
	/* 1 for dots/inch, or 2 for dots/cm.  Note that the pixel aspect */
	/* ratio is defined by X_density/Y_density even when density_unit=0. */
	/* JFIF code for pixel size units */
	/* Horizontal pixel density */
	/* Vertical pixel density */
	/* should an Adobe marker be written? */
	/* State variable: index of next scanline to be written to
	 * jpeg_write_scanlines().  Application may use this to control its
	 * processing loop, e.g., "while (next_scanline < image_height)".
	 */
	/* 0 .. image_height-1  */
	/* Remaining fields are known throughout compressor, but generally
	 * should not be touched by a surrounding application.
	 */
	/*
	 * These fields are computed during compression startup
	 */
	/* TRUE if scan script uses progressive mode */
	/* largest h_samp_factor */
	/* largest v_samp_factor */
	/* # of iMCU rows to be input to coef ctlr */
	/* The coefficient controller receives data in units of MCU rows as defined
	 * for fully interleaved scans (whether the JPEG file is interleaved or not).
	 * There are v_samp_factor * DCTSIZE sample rows of each component in an
	 * "iMCU" (interleaved MCU) row.
	 */
	/*
	 * These fields are valid during any one scan.
	 * They describe the components and MCUs actually appearing in the scan.
	 */
	/* # of JPEG components in this scan */
	/* *cur_comp_info[i] describes component that appears i'th in SOS */
	/* # of MCUs across the image */
	/* # of MCU rows in the image */
	/* # of DCT blocks per MCU */
	/* MCU_membership[i] is index in cur_comp_info of component owning */
	/* i'th block in an MCU */
	/* progressive JPEG parameters for scan */
	/*
	 * Links to compression subobjects (methods and private variables of modules)
	 */
	/* workspace for jpeg_simple_progression */
	/* Master record for a decompression instance */
	/* Fields shared with jpeg_compress_struct */
	/* Source of compressed data */
	/* Basic description of image --- filled in by jpeg_read_header(). */
	/* Application may inspect these values to decide how to process image. */
	/* nominal image width (from SOF marker) */
	/* nominal image height */
	/* # of color components in JPEG image */
	/* colorspace of JPEG image */
	/* Decompression processing parameters --- these fields must be set before
	 * calling jpeg_start_decompress().  Note that jpeg_read_header() initializes
	 * them to default values.
	 */
	/* colorspace for output */
	/* fraction by which to scale image */
	/* image gamma wanted in output */
	/* TRUE=multiple output passes */
	/* TRUE=downsampled data wanted */
	/* IDCT algorithm selector */
	/* TRUE=apply fancy upsampling */
	/* TRUE=apply interblock smoothing */
	/* TRUE=colormapped output wanted */
	/* the following are ignored if not quantize_colors: */
	/* type of color dithering to use */
	/* TRUE=use two-pass color quantization */
	/* max # colors to use in created colormap */
	/* these are significant only in buffered-image mode: */
	/* enable future use of 1-pass quantizer */
	/* enable future use of external colormap */
	/* enable future use of 2-pass quantizer */
	/* Description of actual output image that will be returned to application.
	 * These fields are computed by jpeg_start_decompress().
	 * You can also use jpeg_calc_output_dimensions() to determine these values
	 * in advance of calling jpeg_start_decompress().
	 */
	/* scaled image width */
	/* scaled image height */
	/* # of color components in out_color_space */
	/* # of color components returned */
	/* output_components is 1 (a colormap index) when quantizing colors;
	 * otherwise it equals out_color_components.
	 */
	/* min recommended height of scanline buffer */
	/* If the buffer passed to jpeg_read_scanlines() is less than this many rows
	 * high, space and time will be wasted due to unnecessary data copying.
	 * Usually rec_outbuf_height will be 1 or 2, at most 4.
	 */
	/* When quantizing colors, the output colormap is described by these fields.
	 * The application can supply a colormap by setting colormap non-NULL before
	 * calling jpeg_start_decompress; otherwise a colormap is created during
	 * jpeg_start_decompress or jpeg_start_output.
	 * The map has out_color_components rows and actual_number_of_colors columns.
	 */
	/* number of entries in use */
	/* The color map as a 2-D pixel array */
	/* State variables: these variables indicate the progress of decompression.
	 * The application may examine these but must not modify them.
	 */
	/* Row index of next scanline to be read from jpeg_read_scanlines().
	 * Application may use this to control its processing loop, e.g.,
	 * "while (output_scanline < output_height)".
	 */
	/* 0 .. output_height-1  */
	/* Current input scan number and number of iMCU rows completed in scan.
	 * These indicate the progress of the decompressor input side.
	 */
	/* Number of SOS markers seen so far */
	/* Number of iMCU rows completed */
	/* The "output scan number" is the notional scan being displayed by the
	 * output side.  The decompressor will not allow output scan/row number
	 * to get ahead of input scan/row, but it can fall arbitrarily far behind.
	 */
	/* Nominal scan number being displayed */
	/* Number of iMCU rows read */
	/* Current progression status.  coef_bits[c][i] indicates the precision
	 * with which component c's DCT coefficient i (in zigzag order) is known.
	 * It is -1 when no data has yet been received, otherwise it is the point
	 * transform (shift) value for the most recent scan of the coefficient
	 * (thus, 0 at completion of the progression).
	 * This pointer is NULL when reading a non-progressive file.
	 */
	/* -1 or current Al value for each coef */
	/* Internal JPEG parameters --- the application usually need not look at
	 * these fields.  Note that the decompressor output side may not use
	 * any parameters that can change between scans.
	 */
	/* Quantization and Huffman tables are carried forward across input
	 * datastreams when processing abbreviated JPEG datastreams.
	 */
	/* ptrs to coefficient quantization tables, or NULL if not defined */
	/* ptrs to Huffman coding tables, or NULL if not defined */
	/* These parameters are never carried across datastreams, since they
	 * are given in SOF/SOS markers or defined to be reset by SOI.
	 */
	/* bits of precision in image data */
	/* comp_info[i] describes component that appears i'th in SOF */
	/* TRUE if SOFn specifies progressive mode */
	/* TRUE=arithmetic coding, FALSE=Huffman */
	/* L values for DC arith-coding tables */
	/* U values for DC arith-coding tables */
	/* Kx values for AC arith-coding tables */
	/* MCUs per restart interval, or 0 for no restart */
	/* These fields record data obtained from optional markers recognized by
	 * the JPEG library.
	 */
	/* TRUE iff a JFIF APP0 marker was found */
	/* Data copied from JFIF marker; only valid if saw_JFIF_marker is TRUE: */
	/* JFIF version number */
	/* JFIF code for pixel size units */
	/* Horizontal pixel density */
	/* Vertical pixel density */
	/* TRUE iff an Adobe APP14 marker was found */
	/* Color transform code from Adobe marker */
	/* TRUE=first samples are cosited */
	/* Aside from the specific data retained from APPn markers known to the
	 * library, the uninterpreted contents of any or all APPn and COM markers
	 * can be saved in a list for examination by the application.
	 */
	/* Head of list of saved markers */
	/* Remaining fields are known throughout decompressor, but generally
	 * should not be touched by a surrounding application.
	 */
	/*
	 * These fields are computed during decompression startup
	 */
	/* largest h_samp_factor */
	/* largest v_samp_factor */
	/* smallest DCT_scaled_size of any component */
	/* # of iMCU rows in image */
	/* The coefficient controller's input and output progress is measured in
	 * units of "iMCU" (interleaved MCU) rows.  These are the same as MCU rows
	 * in fully interleaved JPEG scans, but are used whether the scan is
	 * interleaved or not.  We define an iMCU row as v_samp_factor DCT block
	 * rows of each component.  Therefore, the IDCT output contains
	 * v_samp_factor*DCT_[v_]scaled_size sample rows of a component per iMCU row.
	 */
	/* table for fast range-limiting */
	/*
	 * These fields are valid during any one scan.
	 * They describe the components and MCUs actually appearing in the scan.
	 * Note that the decompressor output side must not use these fields.
	 */
	/* # of JPEG components in this scan */
	/* *cur_comp_info[i] describes component that appears i'th in SOS */
	/* # of MCUs across the image */
	/* # of MCU rows in the image */
	/* # of DCT blocks per MCU */
	/* MCU_membership[i] is index in cur_comp_info of component owning */
	/* i'th block in an MCU */
	/* progressive JPEG parameters for scan */
	/* This field is shared between entropy decoder and marker parser.
	 * It is either zero or the code of a JPEG marker that has been
	 * read from the data source, but has not yet been processed.
	 */
	/*
	 * Links to decompression subobjects (methods, private variables of modules)
	 */
	/* "Object" declarations for JPEG modules that may be supplied or called
	 * directly by the surrounding application.
	 * As with all objects in the JPEG library, these structs only define the
	 * publicly visible methods and state variables of a module.  Additional
	 * private fields may exist after the public ones.
	 */
	/* Error handler object */
	/* Error exit handler: does not return to caller */
	/* Conditionally emit a trace or warning message */
	/* Routine that actually outputs a trace or error message */
	/* Format a message string for the most recent JPEG error or message */
	/* recommended size of format_message buffer */
	/* Reset error state variables at start of a new image */
	/* The message ID code and any parameters are saved here.
	 * A message can have one string parameter or up to 8 int parameters.
	 */
	/* Standard state variables for error facility */
	/* max msg_level that will be displayed */
	/* For recoverable corrupt-data errors, we emit a warning message,
	 * but keep going unless emit_message chooses to abort.  emit_message
	 * should count warnings in num_warnings.  The surrounding application
	 * can check for bad data by seeing if num_warnings is nonzero at the
	 * end of processing.
	 */
	/* number of corrupt-data warnings */
	/* These fields point to the table(s) of error message strings.
	 * An application can change the table pointer to switch to a different
	 * message list (typically, to change the language in which errors are
	 * reported).  Some applications may wish to add additional error codes
	 * that will be handled by the JPEG library error mechanism; the second
	 * table pointer is used for this purpose.
	 *
	 * First table includes all errors generated by JPEG library itself.
	 * Error code 0 is reserved for a "no such error string" message.
	 */
	/* Library errors */
	/* Table contains strings 0..last_jpeg_message */
	/* Second table can be added by application (see cjpeg/djpeg for example).
	 * It contains strings numbered first_addon_message..last_addon_message.
	 */
	/* Non-library errors */
	/* code for first string in addon table */
	/* code for last string in addon table */
	/* Progress monitor object */
	/* work units completed in this pass */
	/* total number of work units in this pass */
	/* passes completed so far */
	/* total number of passes expected */
	/* Data destination object for compression */
	/* => next byte to write in buffer */
	/* # of byte spaces remaining in buffer */
	/* Data source object for decompression */
	/* => next byte to read from buffer */
	/* # of bytes remaining in buffer */
	/* Memory manager object.
	 * Allocates "small" objects (a few K total), "large" objects (tens of K),
	 * and "really big" objects (virtual arrays with backing store if needed).
	 * The memory manager does not allow individual objects to be freed; rather,
	 * each created object is assigned to a pool, and whole pools can be freed
	 * at once.  This is faster and more convenient than remembering exactly what
	 * to free, especially where malloc()/free() are not too speedy.
	 * NB: alloc routines never return NULL.  They exit to error_exit if not
	 * successful.
	 */
	/* lasts until master record is destroyed */
	/* lasts until done with image/datastream */

	pub type jvirt_barray_ptr = *mut crate::jpeglib_h::jvirt_barray_control;

	pub type jvirt_sarray_ptr = *mut crate::jpeglib_h::jvirt_sarray_control;

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_error_mgr {
		pub error_exit: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub emit_message:
			Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr, _: libc::c_int) -> ()>,
		pub output_message: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub format_message: Option<
			unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr, _: *mut libc::c_char) -> (),
		>,
		pub reset_error_mgr: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_common_ptr) -> ()>,
		pub msg_code: libc::c_int,
		pub msg_parm: crate::jpeglib_h::C2RustUnnamed_1,
		pub trace_level: libc::c_int,
		pub num_warnings: libc::c_long,
		pub jpeg_message_table: *const *const libc::c_char,
		pub last_jpeg_message: libc::c_int,
		pub addon_message_table: *const *const libc::c_char,
		pub first_addon_message: libc::c_int,
		pub last_addon_message: libc::c_int,
	}

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_compress_struct {
		pub err: *mut crate::jpeglib_h::jpeg_error_mgr,
		pub mem: *mut crate::jpeglib_h::jpeg_memory_mgr,
		pub progress: *mut crate::jpeglib_h::jpeg_progress_mgr,
		pub client_data: *mut libc::c_void,
		pub is_decompressor: crate::jmorecfg_h::boolean,
		pub global_state: libc::c_int,
		pub dest: *mut crate::jpeglib_h::jpeg_destination_mgr,
		pub image_width: crate::jmorecfg_h::JDIMENSION,
		pub image_height: crate::jmorecfg_h::JDIMENSION,
		pub input_components: libc::c_int,
		pub in_color_space: crate::jpeglib_h::J_COLOR_SPACE,
		pub input_gamma: libc::c_double,
		pub data_precision: libc::c_int,
		pub num_components: libc::c_int,
		pub jpeg_color_space: crate::jpeglib_h::J_COLOR_SPACE,
		pub comp_info: *mut crate::jpeglib_h::jpeg_component_info,
		pub quant_tbl_ptrs: [*mut crate::jpeglib_h::JQUANT_TBL; 4],
		pub dc_huff_tbl_ptrs: [*mut crate::jpeglib_h::JHUFF_TBL; 4],
		pub ac_huff_tbl_ptrs: [*mut crate::jpeglib_h::JHUFF_TBL; 4],
		pub arith_dc_L: [crate::jmorecfg_h::UINT8; 16],
		pub arith_dc_U: [crate::jmorecfg_h::UINT8; 16],
		pub arith_ac_K: [crate::jmorecfg_h::UINT8; 16],
		pub num_scans: libc::c_int,
		pub scan_info: *const crate::jpeglib_h::jpeg_scan_info,
		pub raw_data_in: crate::jmorecfg_h::boolean,
		pub arith_code: crate::jmorecfg_h::boolean,
		pub optimize_coding: crate::jmorecfg_h::boolean,
		pub CCIR601_sampling: crate::jmorecfg_h::boolean,
		pub smoothing_factor: libc::c_int,
		pub dct_method: crate::jpeglib_h::J_DCT_METHOD,
		pub restart_interval: libc::c_uint,
		pub restart_in_rows: libc::c_int,
		pub write_JFIF_header: crate::jmorecfg_h::boolean,
		pub JFIF_major_version: crate::jmorecfg_h::UINT8,
		pub JFIF_minor_version: crate::jmorecfg_h::UINT8,
		pub density_unit: crate::jmorecfg_h::UINT8,
		pub X_density: crate::jmorecfg_h::UINT16,
		pub Y_density: crate::jmorecfg_h::UINT16,
		pub write_Adobe_marker: crate::jmorecfg_h::boolean,
		pub next_scanline: crate::jmorecfg_h::JDIMENSION,
		pub progressive_mode: crate::jmorecfg_h::boolean,
		pub max_h_samp_factor: libc::c_int,
		pub max_v_samp_factor: libc::c_int,
		pub total_iMCU_rows: crate::jmorecfg_h::JDIMENSION,
		pub comps_in_scan: libc::c_int,
		pub cur_comp_info: [*mut crate::jpeglib_h::jpeg_component_info; 4],
		pub MCUs_per_row: crate::jmorecfg_h::JDIMENSION,
		pub MCU_rows_in_scan: crate::jmorecfg_h::JDIMENSION,
		pub blocks_in_MCU: libc::c_int,
		pub MCU_membership: [libc::c_int; 10],
		pub Ss: libc::c_int,
		pub Se: libc::c_int,
		pub Ah: libc::c_int,
		pub Al: libc::c_int,
		pub master: *mut crate::jpeglib_h::jpeg_comp_master,
		pub main: *mut crate::jpeglib_h::jpeg_c_main_controller,
		pub prep: *mut crate::jpeglib_h::jpeg_c_prep_controller,
		pub coef: *mut crate::jpeglib_h::jpeg_c_coef_controller,
		pub marker: *mut crate::jpeglib_h::jpeg_marker_writer,
		pub cconvert: *mut crate::jpeglib_h::jpeg_color_converter,
		pub downsample: *mut crate::jpeglib_h::jpeg_downsampler,
		pub fdct: *mut crate::jpeglib_h::jpeg_forward_dct,
		pub entropy: *mut crate::jpeglib_h::jpeg_entropy_encoder,
		pub script_space: *mut crate::jpeglib_h::jpeg_scan_info,
		pub script_space_size: libc::c_int,
	}

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_destination_mgr {
		pub next_output_byte: *mut crate::jmorecfg_h::JOCTET,
		pub free_in_buffer: crate::stddef_h::size_t,
		pub init_destination:
			Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_compress_ptr) -> ()>,
		pub empty_output_buffer: Option<
			unsafe extern "C" fn(_: crate::jpeglib_h::j_compress_ptr) -> crate::jmorecfg_h::boolean,
		>,
		pub term_destination:
			Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_compress_ptr) -> ()>,
	}

	pub type j_compress_ptr = *mut crate::jpeglib_h::jpeg_compress_struct;

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_decompress_struct {
		pub err: *mut crate::jpeglib_h::jpeg_error_mgr,
		pub mem: *mut crate::jpeglib_h::jpeg_memory_mgr,
		pub progress: *mut crate::jpeglib_h::jpeg_progress_mgr,
		pub client_data: *mut libc::c_void,
		pub is_decompressor: crate::jmorecfg_h::boolean,
		pub global_state: libc::c_int,
		pub src: *mut crate::jpeglib_h::jpeg_source_mgr,
		pub image_width: crate::jmorecfg_h::JDIMENSION,
		pub image_height: crate::jmorecfg_h::JDIMENSION,
		pub num_components: libc::c_int,
		pub jpeg_color_space: crate::jpeglib_h::J_COLOR_SPACE,
		pub out_color_space: crate::jpeglib_h::J_COLOR_SPACE,
		pub scale_num: libc::c_uint,
		pub scale_denom: libc::c_uint,
		pub output_gamma: libc::c_double,
		pub buffered_image: crate::jmorecfg_h::boolean,
		pub raw_data_out: crate::jmorecfg_h::boolean,
		pub dct_method: crate::jpeglib_h::J_DCT_METHOD,
		pub do_fancy_upsampling: crate::jmorecfg_h::boolean,
		pub do_block_smoothing: crate::jmorecfg_h::boolean,
		pub quantize_colors: crate::jmorecfg_h::boolean,
		pub dither_mode: crate::jpeglib_h::J_DITHER_MODE,
		pub two_pass_quantize: crate::jmorecfg_h::boolean,
		pub desired_number_of_colors: libc::c_int,
		pub enable_1pass_quant: crate::jmorecfg_h::boolean,
		pub enable_external_quant: crate::jmorecfg_h::boolean,
		pub enable_2pass_quant: crate::jmorecfg_h::boolean,
		pub output_width: crate::jmorecfg_h::JDIMENSION,
		pub output_height: crate::jmorecfg_h::JDIMENSION,
		pub out_color_components: libc::c_int,
		pub output_components: libc::c_int,
		pub rec_outbuf_height: libc::c_int,
		pub actual_number_of_colors: libc::c_int,
		pub colormap: crate::jpeglib_h::JSAMPARRAY,
		pub output_scanline: crate::jmorecfg_h::JDIMENSION,
		pub input_scan_number: libc::c_int,
		pub input_iMCU_row: crate::jmorecfg_h::JDIMENSION,
		pub output_scan_number: libc::c_int,
		pub output_iMCU_row: crate::jmorecfg_h::JDIMENSION,
		pub coef_bits: *mut [libc::c_int; 64],
		pub quant_tbl_ptrs: [*mut crate::jpeglib_h::JQUANT_TBL; 4],
		pub dc_huff_tbl_ptrs: [*mut crate::jpeglib_h::JHUFF_TBL; 4],
		pub ac_huff_tbl_ptrs: [*mut crate::jpeglib_h::JHUFF_TBL; 4],
		pub data_precision: libc::c_int,
		pub comp_info: *mut crate::jpeglib_h::jpeg_component_info,
		pub progressive_mode: crate::jmorecfg_h::boolean,
		pub arith_code: crate::jmorecfg_h::boolean,
		pub arith_dc_L: [crate::jmorecfg_h::UINT8; 16],
		pub arith_dc_U: [crate::jmorecfg_h::UINT8; 16],
		pub arith_ac_K: [crate::jmorecfg_h::UINT8; 16],
		pub restart_interval: libc::c_uint,
		pub saw_JFIF_marker: crate::jmorecfg_h::boolean,
		pub JFIF_major_version: crate::jmorecfg_h::UINT8,
		pub JFIF_minor_version: crate::jmorecfg_h::UINT8,
		pub density_unit: crate::jmorecfg_h::UINT8,
		pub X_density: crate::jmorecfg_h::UINT16,
		pub Y_density: crate::jmorecfg_h::UINT16,
		pub saw_Adobe_marker: crate::jmorecfg_h::boolean,
		pub Adobe_transform: crate::jmorecfg_h::UINT8,
		pub CCIR601_sampling: crate::jmorecfg_h::boolean,
		pub marker_list: crate::jpeglib_h::jpeg_saved_marker_ptr,
		pub max_h_samp_factor: libc::c_int,
		pub max_v_samp_factor: libc::c_int,
		pub min_DCT_scaled_size: libc::c_int,
		pub total_iMCU_rows: crate::jmorecfg_h::JDIMENSION,
		pub sample_range_limit: *mut crate::jmorecfg_h::JSAMPLE,
		pub comps_in_scan: libc::c_int,
		pub cur_comp_info: [*mut crate::jpeglib_h::jpeg_component_info; 4],
		pub MCUs_per_row: crate::jmorecfg_h::JDIMENSION,
		pub MCU_rows_in_scan: crate::jmorecfg_h::JDIMENSION,
		pub blocks_in_MCU: libc::c_int,
		pub MCU_membership: [libc::c_int; 10],
		pub Ss: libc::c_int,
		pub Se: libc::c_int,
		pub Ah: libc::c_int,
		pub Al: libc::c_int,
		pub unread_marker: libc::c_int,
		pub master: *mut crate::jpeglib_h::jpeg_decomp_master,
		pub main: *mut crate::jpeglib_h::jpeg_d_main_controller,
		pub coef: *mut crate::jpeglib_h::jpeg_d_coef_controller,
		pub post: *mut crate::jpeglib_h::jpeg_d_post_controller,
		pub inputctl: *mut crate::jpeglib_h::jpeg_input_controller,
		pub marker: *mut crate::jpeglib_h::jpeg_marker_reader,
		pub entropy: *mut crate::jpeglib_h::jpeg_entropy_decoder,
		pub idct: *mut crate::jpeglib_h::jpeg_inverse_dct,
		pub upsample: *mut crate::jpeglib_h::jpeg_upsampler,
		pub cconvert: *mut crate::jpeglib_h::jpeg_color_deconverter,
		pub cquantize: *mut crate::jpeglib_h::jpeg_color_quantizer,
	}

	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct jpeg_source_mgr {
		pub next_input_byte: *const crate::jmorecfg_h::JOCTET,
		pub bytes_in_buffer: crate::stddef_h::size_t,
		pub init_source: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_decompress_ptr) -> ()>,
		pub fill_input_buffer: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_decompress_ptr,
			) -> crate::jmorecfg_h::boolean,
		>,
		pub skip_input_data: Option<
			unsafe extern "C" fn(_: crate::jpeglib_h::j_decompress_ptr, _: libc::c_long) -> (),
		>,
		pub resync_to_restart: Option<
			unsafe extern "C" fn(
				_: crate::jpeglib_h::j_decompress_ptr,
				_: libc::c_int,
			) -> crate::jmorecfg_h::boolean,
		>,
		pub term_source: Option<unsafe extern "C" fn(_: crate::jpeglib_h::j_decompress_ptr) -> ()>,
	}

	pub type j_decompress_ptr = *mut crate::jpeglib_h::jpeg_decompress_struct;
	/* 5-bit red/6-bit green/5-bit blue */
	/* alpha/red/green/blue */

	pub const JCS_RGB565: crate::jpeglib_h::J_COLOR_SPACE = 16;
	/* alpha/blue/green/red */

	pub const JCS_EXT_ARGB: crate::jpeglib_h::J_COLOR_SPACE = 15;
	/* blue/green/red/alpha */

	pub const JCS_EXT_ABGR: crate::jpeglib_h::J_COLOR_SPACE = 14;
	/* red/green/blue/alpha */

	pub const JCS_EXT_BGRA: crate::jpeglib_h::J_COLOR_SPACE = 13;
	pub const JCS_EXT_RGBA: crate::jpeglib_h::J_COLOR_SPACE = 12;
	/* x/blue/green/red */

	pub const JCS_EXT_XRGB: crate::jpeglib_h::J_COLOR_SPACE = 11;
	/* blue/green/red/x */

	pub const JCS_EXT_XBGR: crate::jpeglib_h::J_COLOR_SPACE = 10;
	/* blue/green/red */

	pub const JCS_EXT_BGRX: crate::jpeglib_h::J_COLOR_SPACE = 9;
	/* red/green/blue/x */

	pub const JCS_EXT_BGR: crate::jpeglib_h::J_COLOR_SPACE = 8;
	/* red/green/blue */

	pub const JCS_EXT_RGBX: crate::jpeglib_h::J_COLOR_SPACE = 7;
	/* Y/Cb/Cr/K */

	pub const JCS_EXT_RGB: crate::jpeglib_h::J_COLOR_SPACE = 6;
	/* C/M/Y/K */

	pub const JCS_YCCK: crate::jpeglib_h::J_COLOR_SPACE = 5;
	/* Y/Cb/Cr (also known as YUV) */

	pub const JCS_CMYK: crate::jpeglib_h::J_COLOR_SPACE = 4;
	pub const JCS_YCbCr: crate::jpeglib_h::J_COLOR_SPACE = 3;
	/* monochrome */

	pub const JCS_RGB: crate::jpeglib_h::J_COLOR_SPACE = 2;
	/* error/unspecified */

	pub const JCS_GRAYSCALE: crate::jpeglib_h::J_COLOR_SPACE = 1;

	pub const JCS_UNKNOWN: crate::jpeglib_h::J_COLOR_SPACE = 0;
	/* floating-point: accurate, fast on fast HW */
	/* faster, less accurate integer method */

	pub const JDCT_FLOAT: crate::jpeglib_h::J_DCT_METHOD = 2;
	/* slow but accurate integer algorithm */

	pub const JDCT_IFAST: crate::jpeglib_h::J_DCT_METHOD = 1;

	pub const JDCT_ISLOW: crate::jpeglib_h::J_DCT_METHOD = 0;
	/* Floyd-Steinberg error diffusion dither */
	/* simple ordered dither */

	pub const JDITHER_FS: crate::jpeglib_h::J_DITHER_MODE = 2;
	/* no dithering */

	pub const JDITHER_ORDERED: crate::jpeglib_h::J_DITHER_MODE = 1;

	pub const JDITHER_NONE: crate::jpeglib_h::J_DITHER_MODE = 0;
	/* TRUE=preprocess input to reduce ringing of edges on white background */
	/* TRUE=optimize quant table in trellis loop */

	pub const JBOOLEAN_OVERSHOOT_DERINGING: crate::jpeglib_h::J_BOOLEAN_PARAM = 1061927929;
	/* TRUE=use scans in trellis optimization */

	pub const JBOOLEAN_TRELLIS_Q_OPT: crate::jpeglib_h::J_BOOLEAN_PARAM = 3777684073;
	/* TRUE=use lambda weighting table */

	pub const JBOOLEAN_USE_SCANS_IN_TRELLIS: crate::jpeglib_h::J_BOOLEAN_PARAM = 4253291573;
	/* TRUE=optimize for sequences of EOB */

	pub const JBOOLEAN_USE_LAMBDA_WEIGHT_TBL: crate::jpeglib_h::J_BOOLEAN_PARAM = 865973855;
	/* TRUE=use trellis quant for DC coefficient */

	pub const JBOOLEAN_TRELLIS_EOB_OPT: crate::jpeglib_h::J_BOOLEAN_PARAM = 3623303040;
	/* TRUE=use trellis quantization */

	pub const JBOOLEAN_TRELLIS_QUANT_DC: crate::jpeglib_h::J_BOOLEAN_PARAM = 865946636;
	/* TRUE=optimize progressive coding scans */

	pub const JBOOLEAN_TRELLIS_QUANT: crate::jpeglib_h::J_BOOLEAN_PARAM = 3306299443;

	pub const JBOOLEAN_OPTIMIZE_SCANS: crate::jpeglib_h::J_BOOLEAN_PARAM = 1745618462;
	/* DC scan optimization mode */
	/* base quantization table index */

	pub const JINT_DC_SCAN_OPT_MODE: crate::jpeglib_h::J_INT_PARAM = 199732540;
	/* number of trellis loops */

	pub const JINT_BASE_QUANT_TBL_IDX: crate::jpeglib_h::J_INT_PARAM = 1145645745;
	/* splitting point for frequency in trellis quantization */

	pub const JINT_TRELLIS_NUM_LOOPS: crate::jpeglib_h::J_INT_PARAM = 3057565497;
	/* compression profile */

	pub const JINT_TRELLIS_FREQ_SPLIT: crate::jpeglib_h::J_INT_PARAM = 1873801511;

	pub const JINT_COMPRESS_PROFILE: crate::jpeglib_h::J_INT_PARAM = 3918628389;
	/* libjpeg[-turbo] defaults (baseline, no mozjpeg extensions) */
	/* best compression ratio (progressive, all mozjpeg extensions) */

	pub const JCP_FASTEST: crate::jpeglib_h::C2RustUnnamed_0 = 720002228;

	pub const JCP_MAX_COMPRESSION: crate::jpeglib_h::C2RustUnnamed_0 = 1560820397;
}
pub mod jmorecfg_h {
	pub type JSAMPLE = libc::c_uchar;
	/* not HAVE_UNSIGNED_CHAR */
	/* HAVE_UNSIGNED_CHAR */
	/* BITS_IN_JSAMPLE == 8 */
	/* BITS_IN_JSAMPLE == 12 */
	/* Representation of a DCT frequency coefficient.
	 * This should be a signed value of at least 16 bits; "short" is usually OK.
	 * Again, we allocate large arrays of these, but you can change to int
	 * if you have memory to burn and "short" is really slow.
	 */

	pub type JCOEF = libc::c_short;
	/* Compressed datastreams are represented as arrays of JOCTET.
	 * These must be EXACTLY 8 bits wide, at least once they are written to
	 * external storage.  Note that when using the stdio data source/destination
	 * managers, this is also the data type passed to fread/fwrite.
	 */

	pub type JOCTET = libc::c_uchar;
	/* not HAVE_UNSIGNED_CHAR */
	/* HAVE_UNSIGNED_CHAR */
	/* These typedefs are used for various table entries and so forth.
	 * They must be at least as wide as specified; but making them too big
	 * won't cost a huge amount of memory, so we don't provide special
	 * extraction code like we did for JSAMPLE.  (In other words, these
	 * typedefs live at a different point on the speed/space tradeoff curve.)
	 */
	/* UINT8 must hold at least the values 0..255. */

	pub type UINT8 = libc::c_uchar;
	/* not HAVE_UNSIGNED_CHAR */
	/* HAVE_UNSIGNED_CHAR */
	/* UINT16 must hold at least the values 0..65535. */

	pub type UINT16 = libc::c_ushort;
	/* Datatype used for image dimensions.  The JPEG standard only supports
	 * images up to 64K*64K due to 16-bit fields in SOF markers.  Therefore
	 * "unsigned int" is sufficient on all machines.  However, if you need to
	 * handle larger images and you don't mind deviating from the spec, you
	 * can change this datatype.  (Note that changing this datatype will
	 * potentially require modifying the SIMD code.  The x86-64 SIMD extensions,
	 * in particular, assume a 32-bit JDIMENSION.)
	 */

	pub type JDIMENSION = libc::c_uint;
	/* a tad under 64K to prevent overflows */
	/* These macros are used in all function definitions and extern declarations.
	 * You could modify them if you need to change function linkage conventions;
	 * in particular, you'll need to do that to make the library a Windows DLL.
	 * Another application is to make all functions global for use with debuggers
	 * or code profilers that require it.
	 */
	/* a function called through method pointers: */
	/* a function used only in its module: */
	/* a function referenced thru EXTERNs: */
	/* a reference to a GLOBAL function: */
	/* Originally, this macro was used as a way of defining function prototypes
	 * for both modern compilers as well as older compilers that did not support
	 * prototype parameters.  libjpeg-turbo has never supported these older,
	 * non-ANSI compilers, but the macro is still included because there is some
	 * software out there that uses it.
	 */
	/* libjpeg-turbo no longer supports platforms that have far symbols (MS-DOS),
	 * but again, some software relies on this macro.
	 */
	/*
	 * On a few systems, type boolean and/or its values FALSE, TRUE may appear
	 * in standard header files.  Or you may have conflicts with application-
	 * specific header files that you want to include together with these files.
	 * Defining HAVE_BOOLEAN before including jpeglib.h should make it work.
	 */

	pub type boolean = libc::c_int;

	pub const TRUE: libc::c_int = 1 as libc::c_int;

	pub const FALSE: libc::c_int = 0 as libc::c_int;
}
pub mod stddef_h {
	pub type size_t = libc::c_ulong;

	pub const NULL: libc::c_int = 0 as libc::c_int;
}
pub mod stdlib {
	extern "C" {
		#[no_mangle]
		pub static mut stdin: *mut crate::stdlib::FILE;

		#[no_mangle]
		pub static mut stdout: *mut crate::stdlib::FILE;

		#[no_mangle]
		pub static mut stderr: *mut crate::stdlib::FILE;

		#[no_mangle]
		pub fn fclose(__stream: *mut crate::stdlib::FILE) -> libc::c_int;

		#[no_mangle]
		pub fn fopen(_: *const libc::c_char, _: *const libc::c_char) -> *mut crate::stdlib::FILE;

		#[no_mangle]
		pub fn fprintf(_: *mut crate::stdlib::FILE, _: *const libc::c_char, _: ...) -> libc::c_int;

		#[no_mangle]
		pub fn sscanf(_: *const libc::c_char, _: *const libc::c_char, _: ...) -> libc::c_int;

		#[no_mangle]
		pub fn fread(
			_: *mut libc::c_void,
			_: libc::c_ulong,
			_: libc::c_ulong,
			_: *mut crate::stdlib::FILE,
		) -> libc::c_ulong;

		#[no_mangle]
		pub fn fwrite(
			_: *const libc::c_void,
			_: libc::c_ulong,
			_: libc::c_ulong,
			_: *mut crate::stdlib::FILE,
		) -> libc::c_ulong;

		#[no_mangle]
		pub fn fseek(
			__stream: *mut crate::stdlib::FILE,
			__off: libc::c_long,
			__whence: libc::c_int,
		) -> libc::c_int;

		#[no_mangle]
		pub fn ftell(__stream: *mut crate::stdlib::FILE) -> libc::c_long;

		#[no_mangle]
		pub fn ferror(__stream: *mut crate::stdlib::FILE) -> libc::c_int;
		#[no_mangle]
		pub fn malloc(_: libc::c_ulong) -> *mut libc::c_void;

		#[no_mangle]
		pub fn realloc(_: *mut libc::c_void, _: libc::c_ulong) -> *mut libc::c_void;

		#[no_mangle]
		pub fn free(__ptr: *mut libc::c_void);

		#[no_mangle]
		pub fn exit(_: libc::c_int) -> !;
		pub type _IO_wide_data;

		pub type _IO_codecvt;

		pub type _IO_marker;
	}
	pub type FILE = crate::stdlib::_IO_FILE;
	pub const SEEK_SET: libc::c_int = 0 as libc::c_int;

	pub const SEEK_END: libc::c_int = 2 as libc::c_int;
	pub const EXIT_SUCCESS: libc::c_int = 0 as libc::c_int;

	pub const EXIT_FAILURE: libc::c_int = 1 as libc::c_int;
	#[repr(C)]
	#[derive(Copy, Clone)]
	pub struct _IO_FILE {
		pub _flags: libc::c_int,
		pub _IO_read_ptr: *mut libc::c_char,
		pub _IO_read_end: *mut libc::c_char,
		pub _IO_read_base: *mut libc::c_char,
		pub _IO_write_base: *mut libc::c_char,
		pub _IO_write_ptr: *mut libc::c_char,
		pub _IO_write_end: *mut libc::c_char,
		pub _IO_buf_base: *mut libc::c_char,
		pub _IO_buf_end: *mut libc::c_char,
		pub _IO_save_base: *mut libc::c_char,
		pub _IO_backup_base: *mut libc::c_char,
		pub _IO_save_end: *mut libc::c_char,
		pub _markers: *mut crate::stdlib::_IO_marker,
		pub _chain: *mut crate::stdlib::_IO_FILE,
		pub _fileno: libc::c_int,
		pub _flags2: libc::c_int,
		pub _old_offset: crate::stdlib::__off_t,
		pub _cur_column: libc::c_ushort,
		pub _vtable_offset: libc::c_schar,
		pub _shortbuf: [libc::c_char; 1],
		pub _lock: *mut libc::c_void,
		pub _offset: crate::stdlib::__off64_t,
		pub _codecvt: *mut crate::stdlib::_IO_codecvt,
		pub _wide_data: *mut crate::stdlib::_IO_wide_data,
		pub _freeres_list: *mut crate::stdlib::_IO_FILE,
		pub _freeres_buf: *mut libc::c_void,
		pub __pad5: crate::stddef_h::size_t,
		pub _mode: libc::c_int,
		pub _unused2: [libc::c_char; 20],
	}

	pub type _IO_lock_t = ();
	pub type __off_t = libc::c_long;

	pub type __off64_t = libc::c_long;
}
use ::mozjpeg::*;

pub use crate::stddef_h::size_t;
pub use crate::stddef_h::NULL;
pub use crate::stdlib::_IO_codecvt;
pub use crate::stdlib::_IO_lock_t;
pub use crate::stdlib::_IO_marker;
pub use crate::stdlib::_IO_wide_data;
pub use crate::stdlib::__off64_t;
pub use crate::stdlib::__off_t;
pub use crate::stdlib::FILE;
pub use crate::stdlib::_IO_FILE;

pub use crate::cdjpeg_h::keymatch;
pub use crate::cdjpeg_h::read_scan_script;
pub use crate::cdjpeg_h::read_stdin;
pub use crate::cdjpeg_h::write_stdout;
pub use crate::cdjpeg_h::EXIT_WARNING;
pub use crate::cdjpeg_h::READ_BINARY;
pub use crate::cdjpeg_h::WRITE_BINARY;
pub use crate::jconfig_h::JPEG_LIB_VERSION;
pub use crate::jmorecfg_h::boolean;
pub use crate::jmorecfg_h::FALSE;
pub use crate::jmorecfg_h::JCOEF;
pub use crate::jmorecfg_h::JDIMENSION;
pub use crate::jmorecfg_h::JOCTET;
pub use crate::jmorecfg_h::JSAMPLE;
pub use crate::jmorecfg_h::TRUE;
pub use crate::jmorecfg_h::UINT16;
pub use crate::jmorecfg_h::UINT8;
pub use crate::jpeglib_h::j_common_ptr;
pub use crate::jpeglib_h::j_compress_ptr;
pub use crate::jpeglib_h::j_decompress_ptr;
pub use crate::jpeglib_h::jpeg_CreateCompress;
pub use crate::jpeglib_h::jpeg_CreateDecompress;
pub use crate::jpeglib_h::jpeg_c_coef_controller;
pub use crate::jpeglib_h::jpeg_c_get_int_param;
pub use crate::jpeglib_h::jpeg_c_int_param_supported;
pub use crate::jpeglib_h::jpeg_c_main_controller;
pub use crate::jpeglib_h::jpeg_c_prep_controller;
pub use crate::jpeglib_h::jpeg_c_set_bool_param;
pub use crate::jpeglib_h::jpeg_c_set_int_param;
pub use crate::jpeglib_h::jpeg_color_converter;
pub use crate::jpeglib_h::jpeg_color_deconverter;
pub use crate::jpeglib_h::jpeg_color_quantizer;
pub use crate::jpeglib_h::jpeg_common_struct;
pub use crate::jpeglib_h::jpeg_comp_master;
pub use crate::jpeglib_h::jpeg_component_info;
pub use crate::jpeglib_h::jpeg_compress_struct;
pub use crate::jpeglib_h::jpeg_copy_critical_parameters;
pub use crate::jpeglib_h::jpeg_d_coef_controller;
pub use crate::jpeglib_h::jpeg_d_main_controller;
pub use crate::jpeglib_h::jpeg_d_post_controller;
pub use crate::jpeglib_h::jpeg_decomp_master;
pub use crate::jpeglib_h::jpeg_decompress_struct;
pub use crate::jpeglib_h::jpeg_destination_mgr;
pub use crate::jpeglib_h::jpeg_destroy_compress;
pub use crate::jpeglib_h::jpeg_destroy_decompress;
pub use crate::jpeglib_h::jpeg_downsampler;
pub use crate::jpeglib_h::jpeg_entropy_decoder;
pub use crate::jpeglib_h::jpeg_entropy_encoder;
pub use crate::jpeglib_h::jpeg_error_mgr;
pub use crate::jpeglib_h::jpeg_finish_compress;
pub use crate::jpeglib_h::jpeg_finish_decompress;
pub use crate::jpeglib_h::jpeg_forward_dct;
pub use crate::jpeglib_h::jpeg_input_controller;
pub use crate::jpeglib_h::jpeg_inverse_dct;
pub use crate::jpeglib_h::jpeg_marker_reader;
pub use crate::jpeglib_h::jpeg_marker_struct;
pub use crate::jpeglib_h::jpeg_marker_writer;
pub use crate::jpeglib_h::jpeg_mem_dest;
pub use crate::jpeglib_h::jpeg_mem_src;
pub use crate::jpeglib_h::jpeg_memory_mgr;
pub use crate::jpeglib_h::jpeg_progress_mgr;
pub use crate::jpeglib_h::jpeg_read_coefficients;
pub use crate::jpeglib_h::jpeg_read_header;
pub use crate::jpeglib_h::jpeg_saved_marker_ptr;
pub use crate::jpeglib_h::jpeg_scan_info;
pub use crate::jpeglib_h::jpeg_simple_progression;
pub use crate::jpeglib_h::jpeg_source_mgr;
pub use crate::jpeglib_h::jpeg_std_error;
pub use crate::jpeglib_h::jpeg_stdio_dest;
pub use crate::jpeglib_h::jpeg_stdio_src;
pub use crate::jpeglib_h::jpeg_upsampler;
pub use crate::jpeglib_h::jpeg_write_coefficients;
pub use crate::jpeglib_h::jpeg_write_icc_profile;
pub use crate::jpeglib_h::jvirt_barray_control;
pub use crate::jpeglib_h::jvirt_barray_ptr;
pub use crate::jpeglib_h::jvirt_sarray_control;
pub use crate::jpeglib_h::jvirt_sarray_ptr;
pub use crate::jpeglib_h::C2RustUnnamed_0;
pub use crate::jpeglib_h::C2RustUnnamed_1;
pub use crate::jpeglib_h::JCS_YCbCr;
pub use crate::jpeglib_h::JBLOCK;
pub use crate::jpeglib_h::JBLOCKARRAY;
pub use crate::jpeglib_h::JBLOCKROW;
pub use crate::jpeglib_h::JBOOLEAN_OPTIMIZE_SCANS;
pub use crate::jpeglib_h::JBOOLEAN_OVERSHOOT_DERINGING;
pub use crate::jpeglib_h::JBOOLEAN_TRELLIS_EOB_OPT;
pub use crate::jpeglib_h::JBOOLEAN_TRELLIS_QUANT;
pub use crate::jpeglib_h::JBOOLEAN_TRELLIS_QUANT_DC;
pub use crate::jpeglib_h::JBOOLEAN_TRELLIS_Q_OPT;
pub use crate::jpeglib_h::JBOOLEAN_USE_LAMBDA_WEIGHT_TBL;
pub use crate::jpeglib_h::JBOOLEAN_USE_SCANS_IN_TRELLIS;
pub use crate::jpeglib_h::JCP_FASTEST;
pub use crate::jpeglib_h::JCP_MAX_COMPRESSION;
pub use crate::jpeglib_h::JCS_CMYK;
pub use crate::jpeglib_h::JCS_EXT_ABGR;
pub use crate::jpeglib_h::JCS_EXT_ARGB;
pub use crate::jpeglib_h::JCS_EXT_BGR;
pub use crate::jpeglib_h::JCS_EXT_BGRA;
pub use crate::jpeglib_h::JCS_EXT_BGRX;
pub use crate::jpeglib_h::JCS_EXT_RGB;
pub use crate::jpeglib_h::JCS_EXT_RGBA;
pub use crate::jpeglib_h::JCS_EXT_RGBX;
pub use crate::jpeglib_h::JCS_EXT_XBGR;
pub use crate::jpeglib_h::JCS_EXT_XRGB;
pub use crate::jpeglib_h::JCS_GRAYSCALE;
pub use crate::jpeglib_h::JCS_RGB;
pub use crate::jpeglib_h::JCS_RGB565;
pub use crate::jpeglib_h::JCS_UNKNOWN;
pub use crate::jpeglib_h::JCS_YCCK;
pub use crate::jpeglib_h::JDCT_FLOAT;
pub use crate::jpeglib_h::JDCT_IFAST;
pub use crate::jpeglib_h::JDCT_ISLOW;
pub use crate::jpeglib_h::JDITHER_FS;
pub use crate::jpeglib_h::JDITHER_NONE;
pub use crate::jpeglib_h::JDITHER_ORDERED;
pub use crate::jpeglib_h::JHUFF_TBL;
pub use crate::jpeglib_h::JINT_BASE_QUANT_TBL_IDX;
pub use crate::jpeglib_h::JINT_COMPRESS_PROFILE;
pub use crate::jpeglib_h::JINT_DC_SCAN_OPT_MODE;
pub use crate::jpeglib_h::JINT_TRELLIS_FREQ_SPLIT;
pub use crate::jpeglib_h::JINT_TRELLIS_NUM_LOOPS;
pub use crate::jpeglib_h::JQUANT_TBL;
pub use crate::jpeglib_h::JSAMPARRAY;
pub use crate::jpeglib_h::JSAMPROW;
pub use crate::jpeglib_h::J_BOOLEAN_PARAM;
pub use crate::jpeglib_h::J_COLOR_SPACE;
pub use crate::jpeglib_h::J_DCT_METHOD;
pub use crate::jpeglib_h::J_DITHER_MODE;
pub use crate::jpeglib_h::J_INT_PARAM;
pub use crate::stdlib::exit;
pub use crate::stdlib::fclose;
pub use crate::stdlib::ferror;
pub use crate::stdlib::fopen;
pub use crate::stdlib::fprintf;
pub use crate::stdlib::fread;
pub use crate::stdlib::free;
pub use crate::stdlib::fseek;
pub use crate::stdlib::ftell;
pub use crate::stdlib::fwrite;
pub use crate::stdlib::malloc;
pub use crate::stdlib::realloc;
pub use crate::stdlib::sscanf;
pub use crate::stdlib::stderr;
pub use crate::stdlib::stdin;
pub use crate::stdlib::stdout;
pub use crate::stdlib::EXIT_FAILURE;
pub use crate::stdlib::EXIT_SUCCESS;
pub use crate::stdlib::SEEK_END;
pub use crate::stdlib::SEEK_SET;
pub use crate::transupp_h::jcopy_markers_execute;
pub use crate::transupp_h::jcopy_markers_setup;
pub use crate::transupp_h::jpeg_transform_info;
pub use crate::transupp_h::jtransform_adjust_parameters;
pub use crate::transupp_h::jtransform_execute_transform;
pub use crate::transupp_h::jtransform_execute_transformation;
pub use crate::transupp_h::jtransform_parse_crop_spec;
pub use crate::transupp_h::jtransform_request_workspace;
pub use crate::transupp_h::JCOPYOPT_ALL;
pub use crate::transupp_h::JCOPYOPT_ALL_EXCEPT_ICC;
pub use crate::transupp_h::JCOPYOPT_COMMENTS;
pub use crate::transupp_h::JCOPYOPT_DEFAULT;
pub use crate::transupp_h::JCOPYOPT_NONE;
pub use crate::transupp_h::JCOPY_OPTION;
pub use crate::transupp_h::JCROP_CODE;
pub use crate::transupp_h::JCROP_FORCE;
pub use crate::transupp_h::JCROP_NEG;
pub use crate::transupp_h::JCROP_POS;
pub use crate::transupp_h::JCROP_UNSET;
pub use crate::transupp_h::JXFORM_CODE;
pub use crate::transupp_h::JXFORM_FLIP_H;
pub use crate::transupp_h::JXFORM_FLIP_V;
pub use crate::transupp_h::JXFORM_NONE;
pub use crate::transupp_h::JXFORM_ROT_180;
pub use crate::transupp_h::JXFORM_ROT_270;
pub use crate::transupp_h::JXFORM_ROT_90;
pub use crate::transupp_h::JXFORM_TRANSPOSE;
pub use crate::transupp_h::JXFORM_TRANSVERSE;

pub use crate::jconfigint_h::BUILD;
pub use crate::jconfigint_h::PACKAGE_NAME;
pub use crate::jconfigint_h::VERSION;
pub use crate::jversion_h::JCOPYRIGHT;
pub use crate::jversion_h::JVERSION;
/*
 * jpegtran.c
 *
 * This file was part of the Independent JPEG Group's software:
 * Copyright (C) 1995-2010, Thomas G. Lane, Guido Vollbeding.
 * libjpeg-turbo Modifications:
 * Copyright (C) 2010, 2014, 2017, D. R. Commander.
 * mozjpeg Modifications:
 * Copyright (C) 2014, Mozilla Corporation.
 * For conditions of distribution and use, see the accompanying README file.
 *
 * This file contains a command-line user interface for JPEG transcoding.
 * It is very similar to cjpeg.c, and partly to djpeg.c, but provides
 * lossless transcoding between different JPEG file formats.  It also
 * provides some lossless and sort-of-lossless transformations of JPEG data.
 */
/* command-line reader for Macintosh */
/*
 * Argument-parsing code.
 * The switch parser is designed to be useful with DOS-style command line
 * syntax, ie, intermixed switches and file names, where only the switches
 * to the left of a given file name affect processing of that file.
 * The main program in this file doesn't actually use this capability...
 */

static mut progname: *const libc::c_char = 0 as *const libc::c_char;
/* program name for error messages */

static mut icc_filename: *mut libc::c_char = 0 as *const libc::c_char as *mut libc::c_char;
/* for -icc switch */

static mut outfilename: *mut libc::c_char = 0 as *const libc::c_char as *mut libc::c_char;
/* for -outfile switch */

static mut prefer_smallest: crate::jmorecfg_h::boolean = 0;
/* use smallest of input or result file (if no image-changing options supplied) */

static mut copyoption: crate::transupp_h::JCOPY_OPTION = crate::transupp_h::JCOPYOPT_NONE;
/* -copy switch */

static mut transformoption: crate::transupp_h::jpeg_transform_info =
	crate::transupp_h::jpeg_transform_info {
		transform: crate::transupp_h::JXFORM_NONE,
		perfect: 0,
		trim: 0,
		force_grayscale: 0,
		crop: 0,
		slow_hflip: 0,
		crop_width: 0,
		crop_width_set: crate::transupp_h::JCROP_UNSET,
		crop_height: 0,
		crop_height_set: crate::transupp_h::JCROP_UNSET,
		crop_xoffset: 0,
		crop_xoffset_set: crate::transupp_h::JCROP_UNSET,
		crop_yoffset: 0,
		crop_yoffset_set: crate::transupp_h::JCROP_UNSET,
		num_components: 0,
		workspace_coef_arrays: 0 as *const crate::jpeglib_h::jvirt_barray_ptr
			as *mut crate::jpeglib_h::jvirt_barray_ptr,
		output_width: 0,
		output_height: 0,
		x_crop_offset: 0,
		y_crop_offset: 0,
		iMCU_sample_width: 0,
		iMCU_sample_height: 0,
	};
/* image transformation options */
#[no_mangle]

pub static mut memsrc: crate::jmorecfg_h::boolean = crate::jmorecfg_h::FALSE;
/* for -memsrc switch */

pub const INPUT_BUF_SIZE: libc::c_int = 4096 as libc::c_int;

unsafe extern "C" fn usage()
/* complain about bad command line */
{
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"usage: %s [switches] \x00" as *const u8 as *const libc::c_char,
		progname,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"[inputfile]\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"Switches (names may be abbreviated):\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -copy none     Copy no extra markers from source file\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -copy comments Copy only comment markers (default)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -copy all      Copy all extra markers\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(crate::stdlib::stderr,
			b"  -optimize      Optimize Huffman table (smaller file, but slow compression, enabled by default)\n\x00"
				as *const u8 as *const libc::c_char);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -progressive   Create progressive JPEG file (enabled by default)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -revert        Revert to standard defaults (instead of mozjpeg defaults)\n\x00"
			as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -fastcrush     Disable progressive scan optimization\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"Switches for modifying the image:\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -crop WxH+X+Y  Crop to a rectangular subarea\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -grayscale     Reduce to grayscale (omit color data)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -flip [horizontal|vertical]  Mirror image (left-right or top-bottom)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -perfect       Fail if there is non-transformable edge blocks\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -rotate [90|180|270]         Rotate image (degrees clockwise)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -transpose     Transpose image\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -transverse    Transverse transpose image\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -trim          Drop non-transformable edge blocks\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"Switches for advanced users:\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -icc FILE      Embed ICC profile contained in FILE\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -restart N     Set restart interval in rows, or in blocks with B\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -maxmemory N   Maximum memory to use (in kbytes)\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -outfile name  Specify name for output file\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -verbose  or  -debug   Emit debug output\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -version       Print version information and exit\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"Switches for wizards:\n\x00" as *const u8 as *const libc::c_char,
	);
	crate::stdlib::fprintf(
		crate::stdlib::stderr,
		b"  -scans FILE    Create multi-scan JPEG per script FILE\n\x00" as *const u8
			as *const libc::c_char,
	);
	crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
}

unsafe extern "C" fn select_transform(mut transform: crate::transupp_h::JXFORM_CODE)
/* Silly little routine to detect multiple transform options,
 * which we can't handle.
 */
{
	if transformoption.transform as libc::c_uint
		== crate::transupp_h::JXFORM_NONE as libc::c_int as libc::c_uint
		|| transformoption.transform as libc::c_uint == transform as libc::c_uint
	{
		transformoption.transform = transform
	} else {
		crate::stdlib::fprintf(
			crate::stdlib::stderr,
			b"%s: can only do one image transformation at a time\n\x00" as *const u8
				as *const libc::c_char,
			progname,
		);
		usage();
	};
}

unsafe extern "C" fn parse_switches(
	mut cinfo: crate::jpeglib_h::j_compress_ptr,
	mut argc: libc::c_int,
	mut argv: *mut *mut libc::c_char,
	mut last_file_arg_seen: libc::c_int,
	mut for_real: crate::jmorecfg_h::boolean,
) -> libc::c_int
/* Parse optional switches.
 * Returns argv[] index of first file-name argument (== argc if none).
 * Any file names with indexes <= last_file_arg_seen are ignored;
 * they have presumably been processed in a previous iteration.
 * (Pass 0 for last_file_arg_seen on the first or only iteration.)
 * for_real is FALSE on the first (dummy) pass; we may skip any expensive
 * processing.
 */ {
	let mut argn: libc::c_int = 0; /* saves -scans parm if any */
	let mut arg: *mut libc::c_char = 0 as *mut libc::c_char;
	let mut simple_progressive: crate::jmorecfg_h::boolean = 0;
	let mut scansarg: *mut libc::c_char = crate::stddef_h::NULL as *mut libc::c_char;
	/* Set up default JPEG parameters. */
	simple_progressive = if (*cinfo).num_scans == 0 as libc::c_int {
		crate::jmorecfg_h::FALSE
	} else {
		crate::jmorecfg_h::TRUE
	};
	icc_filename = crate::stddef_h::NULL as *mut libc::c_char;
	outfilename = crate::stddef_h::NULL as *mut libc::c_char;
	copyoption = crate::transupp_h::JCOPYOPT_DEFAULT as crate::transupp_h::JCOPY_OPTION;
	transformoption.transform = crate::transupp_h::JXFORM_NONE;
	transformoption.perfect = crate::jmorecfg_h::FALSE;
	transformoption.trim = crate::jmorecfg_h::FALSE;
	transformoption.force_grayscale = crate::jmorecfg_h::FALSE;
	transformoption.crop = crate::jmorecfg_h::FALSE;
	transformoption.slow_hflip = crate::jmorecfg_h::FALSE;
	(*(*cinfo).err).trace_level = 0 as libc::c_int;
	prefer_smallest = crate::jmorecfg_h::TRUE;
	/* Scan command line options, adjust parameters */
	argn = 1 as libc::c_int;
	while argn < argc {
		arg = *argv.offset(argn as isize);
		if *arg as libc::c_int != '-' as i32 {
			/* Not a switch, must be a file name argument */
			if !(argn <= last_file_arg_seen) {
				break; /* -outfile applies to just one input file */
			}
			outfilename = crate::stddef_h::NULL as *mut libc::c_char
		/* ignore this name if previously processed */
		/* else done parsing switches */
		} else {
			arg = arg.offset(1); /* advance past switch marker character */
			if crate::cdjpeg_h::keymatch(
				arg,
				b"arithmetic\x00" as *const u8 as *const libc::c_char,
				1 as libc::c_int,
			) != 0
			{
				/* Use arithmetic coding. */
				crate::stdlib::fprintf(
					crate::stdlib::stderr,
					b"%s: sorry, arithmetic coding not supported\n\x00" as *const u8
						as *const libc::c_char,
					progname,
				);
				crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
			} else {
				if crate::cdjpeg_h::keymatch(
					arg,
					b"copy\x00" as *const u8 as *const libc::c_char,
					2 as libc::c_int,
				) != 0
				{
					/* Select which extra markers to copy. */
					argn += 1;
					if argn >= argc {
						/* advance to next argument */
						usage();
					}
					if crate::cdjpeg_h::keymatch(
						*argv.offset(argn as isize),
						b"none\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						copyoption = crate::transupp_h::JCOPYOPT_NONE
					} else if crate::cdjpeg_h::keymatch(
						*argv.offset(argn as isize),
						b"comments\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						copyoption = crate::transupp_h::JCOPYOPT_COMMENTS
					} else if crate::cdjpeg_h::keymatch(
						*argv.offset(argn as isize),
						b"all\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						copyoption = crate::transupp_h::JCOPYOPT_ALL
					} else {
						usage();
					}
				} else if crate::cdjpeg_h::keymatch(
					arg,
					b"crop\x00" as *const u8 as *const libc::c_char,
					2 as libc::c_int,
				) != 0
				{
					/* Perform lossless cropping. */
					argn += 1;
					if argn >= argc {
						/* advance to next argument */
						usage();
					}
					if crate::transupp_h::jtransform_parse_crop_spec(
						&mut transformoption,
						*argv.offset(argn as isize),
					) == 0
					{
						crate::stdlib::fprintf(
							crate::stdlib::stderr,
							b"%s: bogus -crop argument \'%s\'\n\x00" as *const u8
								as *const libc::c_char,
							progname,
							*argv.offset(argn as isize),
						);
						crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
					}
					prefer_smallest = crate::jmorecfg_h::FALSE
				} else if crate::cdjpeg_h::keymatch(
					arg,
					b"debug\x00" as *const u8 as *const libc::c_char,
					1 as libc::c_int,
				) != 0
					|| crate::cdjpeg_h::keymatch(
						arg,
						b"verbose\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
				{
					/* Enable debug printouts. */
					/* On first -d, print version identification */
					static mut printed_version: crate::jmorecfg_h::boolean =
						crate::jmorecfg_h::FALSE;
					if printed_version == 0 {
						crate::stdlib::fprintf(
							crate::stdlib::stderr,
							b"%s version %s (build %s)\n\x00" as *const u8 as *const libc::c_char,
							crate::jconfigint_h::PACKAGE_NAME.as_ptr(),
							crate::jconfigint_h::VERSION.as_ptr(),
							crate::jconfigint_h::BUILD.as_ptr(),
						);
						crate::stdlib::fprintf(
							crate::stdlib::stderr,
							b"%s\n\n\x00" as *const u8 as *const libc::c_char,
							crate::jversion_h::JCOPYRIGHT.as_ptr(),
						);
						crate::stdlib::fprintf(
							crate::stdlib::stderr,
							b"Emulating The Independent JPEG Group\'s software, version %s\n\n\x00"
								as *const u8 as *const libc::c_char,
							crate::jversion_h::JVERSION.as_ptr(),
						);
						printed_version = crate::jmorecfg_h::TRUE
					}
					(*(*cinfo).err).trace_level += 1
				} else if crate::cdjpeg_h::keymatch(
					arg,
					b"version\x00" as *const u8 as *const libc::c_char,
					4 as libc::c_int,
				) != 0
				{
					crate::stdlib::fprintf(
						crate::stdlib::stderr,
						b"%s version %s (build %s)\n\x00" as *const u8 as *const libc::c_char,
						crate::jconfigint_h::PACKAGE_NAME.as_ptr(),
						crate::jconfigint_h::VERSION.as_ptr(),
						crate::jconfigint_h::BUILD.as_ptr(),
					);
					crate::stdlib::exit(crate::stdlib::EXIT_SUCCESS);
				} else {
					if crate::cdjpeg_h::keymatch(
						arg,
						b"flip\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						/* Mirror left-right or top-bottom. */
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						if crate::cdjpeg_h::keymatch(
							*argv.offset(argn as isize),
							b"horizontal\x00" as *const u8 as *const libc::c_char,
							1 as libc::c_int,
						) != 0
						{
							select_transform(crate::transupp_h::JXFORM_FLIP_H);
						} else if crate::cdjpeg_h::keymatch(
							*argv.offset(argn as isize),
							b"vertical\x00" as *const u8 as *const libc::c_char,
							1 as libc::c_int,
						) != 0
						{
							select_transform(crate::transupp_h::JXFORM_FLIP_V);
						} else {
							usage();
						}
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"fastcrush\x00" as *const u8 as *const libc::c_char,
						4 as libc::c_int,
					) != 0
					{
						crate::jpeglib_h::jpeg_c_set_bool_param(
							cinfo,
							crate::jpeglib_h::JBOOLEAN_OPTIMIZE_SCANS,
							crate::jmorecfg_h::FALSE,
						);
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"grayscale\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
						|| crate::cdjpeg_h::keymatch(
							arg,
							b"greyscale\x00" as *const u8 as *const libc::c_char,
							1 as libc::c_int,
						) != 0
					{
						/* Force to grayscale. */
						transformoption.force_grayscale = crate::jmorecfg_h::TRUE;
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"icc\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						/* Set ICC filename. */
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						icc_filename = *argv.offset(argn as isize)
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"maxmemory\x00" as *const u8 as *const libc::c_char,
						3 as libc::c_int,
					) != 0
					{
						/* Maximum memory in Kb (or Mb with 'm'). */
						let mut lval: libc::c_long = 0;
						let mut ch: libc::c_char = 'x' as i32 as libc::c_char;
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						if crate::stdlib::sscanf(
							*argv.offset(argn as isize),
							b"%ld%c\x00" as *const u8 as *const libc::c_char,
							&mut lval as *mut libc::c_long,
							&mut ch as *mut libc::c_char,
						) < 1 as libc::c_int
						{
							usage();
						}
						if ch as libc::c_int == 'm' as i32 || ch as libc::c_int == 'M' as i32 {
							lval *= 1000 as libc::c_long
						}
						(*(*cinfo).mem).max_memory_to_use = lval * 1000 as libc::c_long
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"optimize\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
						|| crate::cdjpeg_h::keymatch(
							arg,
							b"optimise\x00" as *const u8 as *const libc::c_char,
							1 as libc::c_int,
						) != 0
					{
						/* Enable entropy parm optimization. */
						(*cinfo).optimize_coding = crate::jmorecfg_h::TRUE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"outfile\x00" as *const u8 as *const libc::c_char,
						4 as libc::c_int,
					) != 0
					{
						/* Set output file name. */
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						outfilename = *argv.offset(argn as isize)
					/* save it away for later use */
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"perfect\x00" as *const u8 as *const libc::c_char,
						2 as libc::c_int,
					) != 0
					{
						/* Fail if there is any partial edge MCUs that the transform can't
						 * handle. */
						transformoption.perfect = crate::jmorecfg_h::TRUE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"progressive\x00" as *const u8 as *const libc::c_char,
						2 as libc::c_int,
					) != 0
					{
						/* Select simple progressive mode. */
						simple_progressive = crate::jmorecfg_h::TRUE;
						prefer_smallest = crate::jmorecfg_h::FALSE
					/* We must postpone execution until num_components is known. */
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"restart\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						/* Restart interval in MCU rows (or in MCUs with 'b'). */
						let mut lval_0: libc::c_long = 0;
						let mut ch_0: libc::c_char = 'x' as i32 as libc::c_char;
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						if crate::stdlib::sscanf(
							*argv.offset(argn as isize),
							b"%ld%c\x00" as *const u8 as *const libc::c_char,
							&mut lval_0 as *mut libc::c_long,
							&mut ch_0 as *mut libc::c_char,
						) < 1 as libc::c_int
						{
							usage();
						}
						if lval_0 < 0 as libc::c_int as libc::c_long
							|| lval_0 > 65535 as libc::c_long
						{
							usage();
						}
						if ch_0 as libc::c_int == 'b' as i32 || ch_0 as libc::c_int == 'B' as i32 {
							(*cinfo).restart_interval = lval_0 as libc::c_uint;
							(*cinfo).restart_in_rows = 0 as libc::c_int
						/* else prior '-restart n' overrides me */
						} else {
							(*cinfo).restart_in_rows = lval_0 as libc::c_int
							/* restart_interval will be computed during startup */
						}
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"revert\x00" as *const u8 as *const libc::c_char,
						3 as libc::c_int,
					) != 0
					{
						/* revert to old JPEG default */
						crate::jpeglib_h::jpeg_c_set_int_param(
							cinfo,
							crate::jpeglib_h::JINT_COMPRESS_PROFILE,
							crate::jpeglib_h::JCP_FASTEST as libc::c_int,
						);
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"rotate\x00" as *const u8 as *const libc::c_char,
						2 as libc::c_int,
					) != 0
					{
						/* Rotate 90, 180, or 270 degrees (measured clockwise). */
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						if crate::cdjpeg_h::keymatch(
							*argv.offset(argn as isize),
							b"90\x00" as *const u8 as *const libc::c_char,
							2 as libc::c_int,
						) != 0
						{
							select_transform(crate::transupp_h::JXFORM_ROT_90);
						} else if crate::cdjpeg_h::keymatch(
							*argv.offset(argn as isize),
							b"180\x00" as *const u8 as *const libc::c_char,
							3 as libc::c_int,
						) != 0
						{
							select_transform(crate::transupp_h::JXFORM_ROT_180);
						} else if crate::cdjpeg_h::keymatch(
							*argv.offset(argn as isize),
							b"270\x00" as *const u8 as *const libc::c_char,
							3 as libc::c_int,
						) != 0
						{
							select_transform(crate::transupp_h::JXFORM_ROT_270);
						} else {
							usage();
						}
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"scans\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						/* Set scan script. */
						argn += 1;
						if argn >= argc {
							/* advance to next argument */
							usage();
						}
						prefer_smallest = crate::jmorecfg_h::FALSE;
						scansarg = *argv.offset(argn as isize)
					/* We must postpone reading the file in case -progressive appears. */
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"transpose\x00" as *const u8 as *const libc::c_char,
						1 as libc::c_int,
					) != 0
					{
						/* Transpose (across UL-to-LR axis). */
						select_transform(crate::transupp_h::JXFORM_TRANSPOSE);
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"transverse\x00" as *const u8 as *const libc::c_char,
						6 as libc::c_int,
					) != 0
					{
						/* Transverse transpose (across UR-to-LL axis). */
						select_transform(crate::transupp_h::JXFORM_TRANSVERSE);
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else if crate::cdjpeg_h::keymatch(
						arg,
						b"trim\x00" as *const u8 as *const libc::c_char,
						3 as libc::c_int,
					) != 0
					{
						/* Trim off any partial edge MCUs that the transform can't handle. */
						transformoption.trim = crate::jmorecfg_h::TRUE;
						prefer_smallest = crate::jmorecfg_h::FALSE
					} else {
						usage();
						/* bogus switch */
					}
				}
			}
		}
		argn += 1
	}
	/* Post-switch-scanning cleanup */
	if for_real != 0 {
		if simple_progressive != 0 {
			/* process -progressive; -scans can override */
			crate::jpeglib_h::jpeg_simple_progression(cinfo);
		}
		if !scansarg.is_null() {
			/* process -scans if it was present */
			if crate::cdjpeg_h::read_scan_script(cinfo, scansarg) == 0 {
				usage();
			}
		}
	}
	return argn;
	/* return index of next arg (file name) */
}
/*
 * The main program.
 */

unsafe fn main_0(mut argc: libc::c_int, mut argv: *mut *mut libc::c_char) -> libc::c_int {
	let mut srcinfo: crate::jpeglib_h::jpeg_decompress_struct =
		crate::jpeglib_h::jpeg_decompress_struct {
			err: 0 as *mut crate::jpeglib_h::jpeg_error_mgr,
			mem: 0 as *mut crate::jpeglib_h::jpeg_memory_mgr,
			progress: 0 as *mut crate::jpeglib_h::jpeg_progress_mgr,
			client_data: 0 as *mut libc::c_void,
			is_decompressor: 0,
			global_state: 0,
			src: 0 as *mut crate::jpeglib_h::jpeg_source_mgr,
			image_width: 0,
			image_height: 0,
			num_components: 0,
			jpeg_color_space: crate::jpeglib_h::JCS_UNKNOWN,
			out_color_space: crate::jpeglib_h::JCS_UNKNOWN,
			scale_num: 0,
			scale_denom: 0,
			output_gamma: 0.,
			buffered_image: 0,
			raw_data_out: 0,
			dct_method: crate::jpeglib_h::JDCT_ISLOW,
			do_fancy_upsampling: 0,
			do_block_smoothing: 0,
			quantize_colors: 0,
			dither_mode: crate::jpeglib_h::JDITHER_NONE,
			two_pass_quantize: 0,
			desired_number_of_colors: 0,
			enable_1pass_quant: 0,
			enable_external_quant: 0,
			enable_2pass_quant: 0,
			output_width: 0,
			output_height: 0,
			out_color_components: 0,
			output_components: 0,
			rec_outbuf_height: 0,
			actual_number_of_colors: 0,
			colormap: 0 as *mut crate::jpeglib_h::JSAMPROW,
			output_scanline: 0,
			input_scan_number: 0,
			input_iMCU_row: 0,
			output_scan_number: 0,
			output_iMCU_row: 0,
			coef_bits: 0 as *mut [libc::c_int; 64],
			quant_tbl_ptrs: [0 as *mut crate::jpeglib_h::JQUANT_TBL; 4],
			dc_huff_tbl_ptrs: [0 as *mut crate::jpeglib_h::JHUFF_TBL; 4],
			ac_huff_tbl_ptrs: [0 as *mut crate::jpeglib_h::JHUFF_TBL; 4],
			data_precision: 0,
			comp_info: 0 as *mut crate::jpeglib_h::jpeg_component_info,
			progressive_mode: 0,
			arith_code: 0,
			arith_dc_L: [0; 16],
			arith_dc_U: [0; 16],
			arith_ac_K: [0; 16],
			restart_interval: 0,
			saw_JFIF_marker: 0,
			JFIF_major_version: 0,
			JFIF_minor_version: 0,
			density_unit: 0,
			X_density: 0,
			Y_density: 0,
			saw_Adobe_marker: 0,
			Adobe_transform: 0,
			CCIR601_sampling: 0,
			marker_list: 0 as *mut crate::jpeglib_h::jpeg_marker_struct,
			max_h_samp_factor: 0,
			max_v_samp_factor: 0,
			min_DCT_scaled_size: 0,
			total_iMCU_rows: 0,
			sample_range_limit: 0 as *mut crate::jmorecfg_h::JSAMPLE,
			comps_in_scan: 0,
			cur_comp_info: [0 as *mut crate::jpeglib_h::jpeg_component_info; 4],
			MCUs_per_row: 0,
			MCU_rows_in_scan: 0,
			blocks_in_MCU: 0,
			MCU_membership: [0; 10],
			Ss: 0,
			Se: 0,
			Ah: 0,
			Al: 0,
			unread_marker: 0,
			master: 0 as *mut crate::jpeglib_h::jpeg_decomp_master,
			main: 0 as *mut crate::jpeglib_h::jpeg_d_main_controller,
			coef: 0 as *mut crate::jpeglib_h::jpeg_d_coef_controller,
			post: 0 as *mut crate::jpeglib_h::jpeg_d_post_controller,
			inputctl: 0 as *mut crate::jpeglib_h::jpeg_input_controller,
			marker: 0 as *mut crate::jpeglib_h::jpeg_marker_reader,
			entropy: 0 as *mut crate::jpeglib_h::jpeg_entropy_decoder,
			idct: 0 as *mut crate::jpeglib_h::jpeg_inverse_dct,
			upsample: 0 as *mut crate::jpeglib_h::jpeg_upsampler,
			cconvert: 0 as *mut crate::jpeglib_h::jpeg_color_deconverter,
			cquantize: 0 as *mut crate::jpeglib_h::jpeg_color_quantizer,
		};
	let mut dstinfo: crate::jpeglib_h::jpeg_compress_struct =
		crate::jpeglib_h::jpeg_compress_struct {
			err: 0 as *mut crate::jpeglib_h::jpeg_error_mgr,
			mem: 0 as *mut crate::jpeglib_h::jpeg_memory_mgr,
			progress: 0 as *mut crate::jpeglib_h::jpeg_progress_mgr,
			client_data: 0 as *mut libc::c_void,
			is_decompressor: 0,
			global_state: 0,
			dest: 0 as *mut crate::jpeglib_h::jpeg_destination_mgr,
			image_width: 0,
			image_height: 0,
			input_components: 0,
			in_color_space: crate::jpeglib_h::JCS_UNKNOWN,
			input_gamma: 0.,
			data_precision: 0,
			num_components: 0,
			jpeg_color_space: crate::jpeglib_h::JCS_UNKNOWN,
			comp_info: 0 as *mut crate::jpeglib_h::jpeg_component_info,
			quant_tbl_ptrs: [0 as *mut crate::jpeglib_h::JQUANT_TBL; 4],
			dc_huff_tbl_ptrs: [0 as *mut crate::jpeglib_h::JHUFF_TBL; 4],
			ac_huff_tbl_ptrs: [0 as *mut crate::jpeglib_h::JHUFF_TBL; 4],
			arith_dc_L: [0; 16],
			arith_dc_U: [0; 16],
			arith_ac_K: [0; 16],
			num_scans: 0,
			scan_info: 0 as *const crate::jpeglib_h::jpeg_scan_info,
			raw_data_in: 0,
			arith_code: 0,
			optimize_coding: 0,
			CCIR601_sampling: 0,
			smoothing_factor: 0,
			dct_method: crate::jpeglib_h::JDCT_ISLOW,
			restart_interval: 0,
			restart_in_rows: 0,
			write_JFIF_header: 0,
			JFIF_major_version: 0,
			JFIF_minor_version: 0,
			density_unit: 0,
			X_density: 0,
			Y_density: 0,
			write_Adobe_marker: 0,
			next_scanline: 0,
			progressive_mode: 0,
			max_h_samp_factor: 0,
			max_v_samp_factor: 0,
			total_iMCU_rows: 0,
			comps_in_scan: 0,
			cur_comp_info: [0 as *mut crate::jpeglib_h::jpeg_component_info; 4],
			MCUs_per_row: 0,
			MCU_rows_in_scan: 0,
			blocks_in_MCU: 0,
			MCU_membership: [0; 10],
			Ss: 0,
			Se: 0,
			Ah: 0,
			Al: 0,
			master: 0 as *mut crate::jpeglib_h::jpeg_comp_master,
			main: 0 as *mut crate::jpeglib_h::jpeg_c_main_controller,
			prep: 0 as *mut crate::jpeglib_h::jpeg_c_prep_controller,
			coef: 0 as *mut crate::jpeglib_h::jpeg_c_coef_controller,
			marker: 0 as *mut crate::jpeglib_h::jpeg_marker_writer,
			cconvert: 0 as *mut crate::jpeglib_h::jpeg_color_converter,
			downsample: 0 as *mut crate::jpeglib_h::jpeg_downsampler,
			fdct: 0 as *mut crate::jpeglib_h::jpeg_forward_dct,
			entropy: 0 as *mut crate::jpeglib_h::jpeg_entropy_encoder,
			script_space: 0 as *mut crate::jpeglib_h::jpeg_scan_info,
			script_space_size: 0,
		};
	let mut jsrcerr: crate::jpeglib_h::jpeg_error_mgr = crate::jpeglib_h::jpeg_error_mgr {
		error_exit: None,
		emit_message: None,
		output_message: None,
		format_message: None,
		reset_error_mgr: None,
		msg_code: 0,
		msg_parm: crate::jpeglib_h::C2RustUnnamed_1 { i: [0; 8] },
		trace_level: 0,
		num_warnings: 0,
		jpeg_message_table: 0 as *const *const libc::c_char,
		last_jpeg_message: 0,
		addon_message_table: 0 as *const *const libc::c_char,
		first_addon_message: 0,
		last_addon_message: 0,
	};
	let mut jdsterr: crate::jpeglib_h::jpeg_error_mgr = crate::jpeglib_h::jpeg_error_mgr {
		error_exit: None,
		emit_message: None,
		output_message: None,
		format_message: None,
		reset_error_mgr: None,
		msg_code: 0,
		msg_parm: crate::jpeglib_h::C2RustUnnamed_1 { i: [0; 8] },
		trace_level: 0,
		num_warnings: 0,
		jpeg_message_table: 0 as *const *const libc::c_char,
		last_jpeg_message: 0,
		addon_message_table: 0 as *const *const libc::c_char,
		first_addon_message: 0,
		last_addon_message: 0,
	};
	let mut src_coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr =
		0 as *mut crate::jpeglib_h::jvirt_barray_ptr;
	let mut dst_coef_arrays: *mut crate::jpeglib_h::jvirt_barray_ptr =
		0 as *mut crate::jpeglib_h::jvirt_barray_ptr;
	let mut file_index: libc::c_int = 0;
	/* We assume all-in-memory processing and can therefore use only a
	 * single file pointer for sequential input and output operation.
	 */
	let mut fp: *mut crate::stdlib::FILE = 0 as *mut crate::stdlib::FILE;
	let mut inbuffer: *mut libc::c_uchar = crate::stddef_h::NULL as *mut libc::c_uchar;
	let mut insize: libc::c_ulong = 0 as libc::c_int as libc::c_ulong;
	let mut outbuffer: *mut libc::c_uchar = crate::stddef_h::NULL as *mut libc::c_uchar;
	let mut outsize: libc::c_ulong = 0 as libc::c_int as libc::c_ulong;
	let mut icc_file: *mut crate::stdlib::FILE = 0 as *mut crate::stdlib::FILE;
	let mut icc_profile: *mut crate::jmorecfg_h::JOCTET =
		crate::stddef_h::NULL as *mut crate::jmorecfg_h::JOCTET;
	let mut icc_len: libc::c_long = 0 as libc::c_int as libc::c_long;
	/* On Mac, fetch a command line. */
	progname = *argv.offset(0 as libc::c_int as isize); /* in case C library doesn't provide it */
	if progname.is_null()
		|| *progname.offset(0 as libc::c_int as isize) as libc::c_int == 0 as libc::c_int
	{
		progname = b"jpegtran\x00" as *const u8 as *const libc::c_char
	}
	/* Initialize the JPEG decompression object with default error handling. */
	srcinfo.err = crate::jpeglib_h::jpeg_std_error(&mut jsrcerr);
	crate::jpeglib_h::jpeg_CreateDecompress(
		&mut srcinfo,
		crate::jconfig_h::JPEG_LIB_VERSION,
		::std::mem::size_of::<crate::jpeglib_h::jpeg_decompress_struct>() as libc::c_ulong,
	);
	/* Initialize the JPEG compression object with default error handling. */
	dstinfo.err = crate::jpeglib_h::jpeg_std_error(&mut jdsterr);
	crate::jpeglib_h::jpeg_CreateCompress(
		&mut dstinfo,
		crate::jconfig_h::JPEG_LIB_VERSION,
		::std::mem::size_of::<crate::jpeglib_h::jpeg_compress_struct>() as libc::c_ulong,
	);
	/* Scan command line to find file names.
	 * It is convenient to use just one switch-parsing routine, but the switch
	 * values read here are mostly ignored; we will rescan the switches after
	 * opening the input file.  Also note that most of the switches affect the
	 * destination JPEG object, so we parse into that and then copy over what
	 * needs to affects the source too.
	 */
	file_index = parse_switches(
		&mut dstinfo,
		argc,
		argv,
		0 as libc::c_int,
		crate::jmorecfg_h::FALSE,
	);
	jsrcerr.trace_level = jdsterr.trace_level;
	(*srcinfo.mem).max_memory_to_use = (*dstinfo.mem).max_memory_to_use;
	/* Unix style: expect zero or one file name */
	if file_index < argc - 1 as libc::c_int {
		crate::stdlib::fprintf(
			crate::stdlib::stderr,
			b"%s: only one input file\n\x00" as *const u8 as *const libc::c_char,
			progname,
		);
		usage();
	}
	/* TWO_FILE_COMMANDLINE */
	/* Open the input file. */
	if file_index < argc {
		fp = crate::stdlib::fopen(
			*argv.offset(file_index as isize),
			crate::cdjpeg_h::READ_BINARY.as_ptr(),
		);
		if fp.is_null() {
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t open %s for reading\n\x00" as *const u8 as *const libc::c_char,
				progname,
				*argv.offset(file_index as isize),
			);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
	} else {
		/* default input file is stdin */
		fp = crate::cdjpeg_h::read_stdin()
	}
	if !icc_filename.is_null() {
		icc_file = crate::stdlib::fopen(icc_filename, crate::cdjpeg_h::READ_BINARY.as_ptr());
		if icc_file.is_null() {
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t open %s\n\x00" as *const u8 as *const libc::c_char,
				progname,
				icc_filename,
			);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
		if crate::stdlib::fseek(
			icc_file,
			0 as libc::c_int as libc::c_long,
			crate::stdlib::SEEK_END,
		) < 0 as libc::c_int
			|| {
				icc_len = crate::stdlib::ftell(icc_file);
				(icc_len) < 1 as libc::c_int as libc::c_long
			}
			|| crate::stdlib::fseek(
				icc_file,
				0 as libc::c_int as libc::c_long,
				crate::stdlib::SEEK_SET,
			) < 0 as libc::c_int
		{
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t determine size of %s\n\x00" as *const u8 as *const libc::c_char,
				progname,
				icc_filename,
			);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
		icc_profile =
			crate::stdlib::malloc(icc_len as libc::c_ulong) as *mut crate::jmorecfg_h::JOCTET;
		if icc_profile.is_null() {
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t allocate memory for ICC profile\n\x00" as *const u8
					as *const libc::c_char,
				progname,
			);
			crate::stdlib::fclose(icc_file);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
		if crate::stdlib::fread(
			icc_profile as *mut libc::c_void,
			icc_len as libc::c_ulong,
			1 as libc::c_int as libc::c_ulong,
			icc_file,
		) < 1 as libc::c_int as libc::c_ulong
		{
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t read ICC profile from %s\n\x00" as *const u8 as *const libc::c_char,
				progname,
				icc_filename,
			);
			crate::stdlib::free(icc_profile as *mut libc::c_void);
			crate::stdlib::fclose(icc_file);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
		crate::stdlib::fclose(icc_file);
		if copyoption as libc::c_uint
			== crate::transupp_h::JCOPYOPT_ALL as libc::c_int as libc::c_uint
		{
			copyoption = crate::transupp_h::JCOPYOPT_ALL_EXCEPT_ICC
		}
	}
	/* Specify data source for decompression */
	if crate::jpeglib_h::jpeg_c_int_param_supported(
		&mut dstinfo,
		crate::jpeglib_h::JINT_COMPRESS_PROFILE,
	) != 0
		&& crate::jpeglib_h::jpeg_c_get_int_param(
			&mut dstinfo,
			crate::jpeglib_h::JINT_COMPRESS_PROFILE,
		) == crate::jpeglib_h::JCP_MAX_COMPRESSION as libc::c_int
	{
		memsrc = crate::jmorecfg_h::TRUE
	} /* needed to revert to original */
	if memsrc != 0 {
		let mut nbytes: crate::stddef_h::size_t = 0;
		loop {
			inbuffer = crate::stdlib::realloc(
				inbuffer as *mut libc::c_void,
				insize.wrapping_add(INPUT_BUF_SIZE as libc::c_ulong),
			) as *mut libc::c_uchar;
			if inbuffer.is_null() {
				crate::stdlib::fprintf(
					crate::stdlib::stderr,
					b"%s: memory allocation failure\n\x00" as *const u8 as *const libc::c_char,
					progname,
				);
				crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
			}
			nbytes = crate::stdlib::fread(
				&mut *inbuffer.offset(insize as isize) as *mut libc::c_uchar as *mut libc::c_void,
				1 as libc::c_int as crate::stddef_h::size_t,
				4096 as libc::c_int as crate::stddef_h::size_t,
				fp,
			);
			if nbytes < INPUT_BUF_SIZE as libc::c_ulong && crate::stdlib::ferror(fp) != 0 {
				if file_index < argc {
					crate::stdlib::fprintf(
						crate::stdlib::stderr,
						b"%s: can\'t read from %s\n\x00" as *const u8 as *const libc::c_char,
						progname,
						*argv.offset(file_index as isize),
					);
				} else {
					crate::stdlib::fprintf(
						crate::stdlib::stderr,
						b"%s: can\'t read from stdin\n\x00" as *const u8 as *const libc::c_char,
						progname,
					);
				}
			}
			insize = insize.wrapping_add(nbytes);
			if !(nbytes == INPUT_BUF_SIZE as libc::c_ulong) {
				break;
			}
		}
		crate::jpeglib_h::jpeg_mem_src(&mut srcinfo, inbuffer, insize);
	} else {
		crate::jpeglib_h::jpeg_stdio_src(&mut srcinfo, fp);
	}
	/* Enable saving of extra markers that we want to copy */
	crate::transupp_h::jcopy_markers_setup(&mut srcinfo, copyoption);
	/* Read file header */
	crate::jpeglib_h::jpeg_read_header(&mut srcinfo, crate::jmorecfg_h::TRUE);
	/* Any space needed by a transform option must be requested before
	 * jpeg_read_coefficients so that memory allocation will be done right.
	 */
	/* Fail right away if -perfect is given and transformation is not perfect.
	 */
	if crate::transupp_h::jtransform_request_workspace(&mut srcinfo, &mut transformoption) == 0 {
		crate::stdlib::fprintf(
			crate::stdlib::stderr,
			b"%s: transformation is not perfect\n\x00" as *const u8 as *const libc::c_char,
			progname,
		);
		crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
	}
	/* Read source file as DCT coefficients */
	src_coef_arrays = crate::jpeglib_h::jpeg_read_coefficients(&mut srcinfo);
	/* Initialize destination compression parameters from source values */
	crate::jpeglib_h::jpeg_copy_critical_parameters(&mut srcinfo, &mut dstinfo);
	/* Adjust destination parameters if required by transform options;
	 * also find out which set of coefficient arrays will hold the output.
	 */
	dst_coef_arrays = crate::transupp_h::jtransform_adjust_parameters(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);
	/* Close input file, if we opened it.
	 * Note: we assume that jpeg_read_coefficients consumed all input
	 * until JPEG_REACHED_EOI, and that jpeg_finish_decompress will
	 * only consume more while (! cinfo->inputctl->eoi_reached).
	 * We cannot call jpeg_finish_decompress here since we still need the
	 * virtual arrays allocated from the source object for processing.
	 */
	if fp != crate::stdlib::stdin {
		crate::stdlib::fclose(fp);
	}
	/* Open the output file. */
	if !outfilename.is_null() {
		fp = crate::stdlib::fopen(outfilename, crate::cdjpeg_h::WRITE_BINARY.as_ptr());
		if fp.is_null() {
			crate::stdlib::fprintf(
				crate::stdlib::stderr,
				b"%s: can\'t open %s for writing\n\x00" as *const u8 as *const libc::c_char,
				progname,
				outfilename,
			);
			crate::stdlib::exit(crate::stdlib::EXIT_FAILURE);
		}
	} else {
		/* default output file is stdout */
		fp = crate::cdjpeg_h::write_stdout()
	}
	/* Adjust default compression parameters by re-parsing the options */
	file_index = parse_switches(
		&mut dstinfo,
		argc,
		argv,
		0 as libc::c_int,
		crate::jmorecfg_h::TRUE,
	);
	/* Specify data destination for compression */
	if crate::jpeglib_h::jpeg_c_int_param_supported(
		&mut dstinfo,
		crate::jpeglib_h::JINT_COMPRESS_PROFILE,
	) != 0
		&& crate::jpeglib_h::jpeg_c_get_int_param(
			&mut dstinfo,
			crate::jpeglib_h::JINT_COMPRESS_PROFILE,
		) == crate::jpeglib_h::JCP_MAX_COMPRESSION as libc::c_int
	{
		crate::jpeglib_h::jpeg_mem_dest(&mut dstinfo, &mut outbuffer, &mut outsize);
	} else {
		crate::jpeglib_h::jpeg_stdio_dest(&mut dstinfo, fp);
	}
	/* Start compressor (note no image data is actually written here) */
	crate::jpeglib_h::jpeg_write_coefficients(&mut dstinfo, dst_coef_arrays);
	/* Copy to the output file any extra markers that we want to preserve */
	crate::transupp_h::jcopy_markers_execute(&mut srcinfo, &mut dstinfo, copyoption);
	if !icc_profile.is_null() {
		crate::jpeglib_h::jpeg_write_icc_profile(
			&mut dstinfo,
			icc_profile,
			icc_len as libc::c_uint,
		);
	}
	/* Execute image transformation, if any */
	crate::transupp_h::jtransform_execute_transform(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);
	/* Finish compression and release memory */
	crate::jpeglib_h::jpeg_finish_compress(&mut dstinfo);
	if crate::jpeglib_h::jpeg_c_int_param_supported(
		&mut dstinfo,
		crate::jpeglib_h::JINT_COMPRESS_PROFILE,
	) != 0
		&& crate::jpeglib_h::jpeg_c_get_int_param(
			&mut dstinfo,
			crate::jpeglib_h::JINT_COMPRESS_PROFILE,
		) == crate::jpeglib_h::JCP_MAX_COMPRESSION as libc::c_int
	{
		let mut nbytes_0: crate::stddef_h::size_t = 0;
		let mut buffer: *mut libc::c_uchar = outbuffer;
		let mut size: libc::c_ulong = outsize;
		if prefer_smallest != 0 && insize < size {
			size = insize;
			buffer = inbuffer
		}
		nbytes_0 = crate::stdlib::fwrite(
			buffer as *const libc::c_void,
			1 as libc::c_int as crate::stddef_h::size_t,
			size,
			fp,
		);
		if nbytes_0 < size && crate::stdlib::ferror(fp) != 0 {
			if file_index < argc {
				crate::stdlib::fprintf(
					crate::stdlib::stderr,
					b"%s: can\'t write to %s\n\x00" as *const u8 as *const libc::c_char,
					progname,
					*argv.offset(file_index as isize),
				);
			} else {
				crate::stdlib::fprintf(
					crate::stdlib::stderr,
					b"%s: can\'t write to stdout\n\x00" as *const u8 as *const libc::c_char,
					progname,
				);
			}
		}
	}
	crate::jpeglib_h::jpeg_destroy_compress(&mut dstinfo);
	crate::jpeglib_h::jpeg_finish_decompress(&mut srcinfo);
	crate::jpeglib_h::jpeg_destroy_decompress(&mut srcinfo);
	/* Close output file, if we opened it */
	if fp != crate::stdlib::stdout {
		crate::stdlib::fclose(fp);
	}
	crate::stdlib::free(inbuffer as *mut libc::c_void);
	crate::stdlib::free(outbuffer as *mut libc::c_void);
	if !icc_profile.is_null() {
		crate::stdlib::free(icc_profile as *mut libc::c_void);
	}
	/* All done. */
	crate::stdlib::exit(if jsrcerr.num_warnings + jdsterr.num_warnings != 0 {
		crate::cdjpeg_h::EXIT_WARNING
	} else {
		crate::stdlib::EXIT_SUCCESS
	});
	/* suppress no-return-value warnings */
}
#[main]
pub fn main() {
	let mut args: Vec<*mut libc::c_char> = Vec::new();
	for arg in ::std::env::args() {
		args.push(
			::std::ffi::CString::new(arg)
				.expect("Failed to convert argument into CString.")
				.into_raw(),
		);
	}
	args.push(::std::ptr::null_mut());
	unsafe {
		::std::process::exit(main_0(
			(args.len() - 1) as libc::c_int,
			args.as_mut_ptr() as *mut *mut libc::c_char,
		) as i32)
	}
}
