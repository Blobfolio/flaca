/*!
# Flaca: Jpegtran

This is essentially a port of the `MozJPEG` code relating to:
```bash
jpegtran -copy none -progressive -optimize
```



## Reference:

The reference materials are a bit all over the place, but the main sources
looked at to bring this all together were:
* [mozjpeg](https://github.com/mozilla/mozjpeg/blob/master/libjpeg.txt)
* [mozjpeg-sys](https://github.com/kornelski/mozjpeg-sys/blob/master/examples/reencode.rs)
* [mozjpeg-rs](https://github.com/immunant/mozjpeg-rs/blob/master/bin/jpegtran.rs)
*/

use mozjpeg_sys::{
	j_compress_ptr,
	j_decompress_ptr,
	JCROP_CODE_JCROP_UNSET,
	jpeg_compress_struct,
	jpeg_copy_critical_parameters,
	jpeg_create_compress,
	jpeg_create_decompress,
	jpeg_decompress_struct,
	jpeg_destroy_compress,
	jpeg_destroy_decompress,
	jpeg_error_mgr,
	jpeg_finish_compress,
	jpeg_finish_decompress,
	jpeg_mem_dest,
	jpeg_mem_src,
	jpeg_read_coefficients,
	jpeg_read_header,
	jpeg_simple_progression,
	jpeg_std_error,
	jpeg_transform_info,
	jpeg_write_coefficients,
	jtransform_adjust_parameters,
	jtransform_execute_transform,
	jtransform_request_workspace,
	jvirt_barray_ptr,
	JXFORM_CODE_JXFORM_NONE,
};
use std::os::raw::{
	c_uchar,
	c_uint,
	c_ulong,
	c_void,
};

// We need a couple more things from jpegtran. Mozjpeg-sys includes the right
// sources but doesn't export the definitions.
extern "C" {
	fn jcopy_markers_setup(srcinfo: j_decompress_ptr, option: c_uint);
	fn jcopy_markers_execute(
		srcinfo: j_decompress_ptr,
		dstinfo: j_compress_ptr,
		option: c_uint,
	);
}



#[allow(unused_assignments)]
/// # Jpegtran (Memory Mode)
///
/// ## Errors
///
/// An error is returned on failure, including cases where everything worked
/// but no compression was possible.
///
/// ## Safety
///
/// The data should be valid JPEG data. Weird things could happen if it isn't.
pub(super) unsafe fn jpegtran_mem(data: &[u8]) -> Option<Vec<u8>> {
	let mut transformoption = jpeg_transform_info {
		transform: JXFORM_CODE_JXFORM_NONE,
		perfect: 0,
		trim: 0,
		force_grayscale: 0,
		crop: 0,
		slow_hflip: 0,
		crop_width: 0,
		crop_width_set: JCROP_CODE_JCROP_UNSET,
		crop_height: 0,
		crop_height_set: JCROP_CODE_JCROP_UNSET,
		crop_xoffset: 0,
		crop_xoffset_set: JCROP_CODE_JCROP_UNSET,
		crop_yoffset: 0,
		crop_yoffset_set: JCROP_CODE_JCROP_UNSET,
		num_components: 0,
		workspace_coef_arrays: std::ptr::null_mut::<jvirt_barray_ptr>(),
		output_width: 0,
		output_height: 0,
		x_crop_offset: 0,
		y_crop_offset: 0,
		iMCU_sample_width: 0,
		iMCU_sample_height: 0,
	};

	let mut jsrcerr: jpeg_error_mgr = std::mem::zeroed();
	let mut srcinfo: jpeg_decompress_struct = std::mem::zeroed();
	srcinfo.common.err = jpeg_std_error(&mut jsrcerr);

	let mut jdsterr: jpeg_error_mgr = std::mem::zeroed();
	let mut dstinfo: jpeg_compress_struct = std::mem::zeroed();
	dstinfo.common.err = jpeg_std_error(&mut jdsterr);

	// Initialize the JPEG (de/)compression object with default error handling.
	jpeg_create_decompress(&mut srcinfo);
	jpeg_create_compress(&mut dstinfo);

	// The trace levels should both be zero, but just in case, let's make sure
	// they're the same.
	jsrcerr.trace_level = jdsterr.trace_level;

	// Load the source file.
	jpeg_mem_src(&mut srcinfo, data.as_ptr(), data.len() as c_ulong);

	// Ignore markers.
	jcopy_markers_setup(&mut srcinfo, 0);

	// Read the file header to get to the goods.
	jpeg_read_header(&mut srcinfo, 1);

	// Read a few more properties into the source struct.
	if jtransform_request_workspace(&mut srcinfo, &mut transformoption) == 0 {
		return None;
	}

	// Read source file as DCT coefficients.
	let src_coef_arrays: *mut jvirt_barray_ptr = jpeg_read_coefficients(&mut srcinfo);

	// Initialize destination compression parameters from source values.
	jpeg_copy_critical_parameters(&srcinfo, &mut dstinfo);

	// Adjust destination parameters if required by transform options, and sync
	// the coefficient arrays.
	let dst_coef_arrays: *mut jvirt_barray_ptr = jtransform_adjust_parameters(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);

	// Get an output buffer going.
	let mut out_ptr: *mut c_uchar = std::ptr::null_mut();
	let mut out_size: c_ulong = 0;

	// Turn on "progressive" and "code optimizing" for the output.
	dstinfo.optimize_coding = 1;
	jpeg_simple_progression(&mut dstinfo);

	// And load the destination file.
	jpeg_mem_dest(&mut dstinfo, &mut out_ptr, &mut out_size);

	// Start the compressor. Note: no data is written here.
	jpeg_write_coefficients(&mut dstinfo, dst_coef_arrays);

	// Make sure we aren't copying any markers.
	jcopy_markers_execute(&mut srcinfo, &mut dstinfo, 0);

	// Execute and write the transformation, if any.
	jtransform_execute_transform(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);

	// Let's get the data!
	jpeg_finish_compress(&mut dstinfo);

	// This library doesn't really have a consistent way of handling errors,
	// but msg_code not changing from its default (of zero) is a reasonable
	// proxy.
	let mut res: bool = 0 == (*dstinfo.common.err).msg_code;

	let out: Vec<u8> =
		if out_ptr.is_null() || out_size == 0 { Vec::new() }
		else {
			let tmp =
				if let Ok(size) = usize::try_from(out_size) {
					std::slice::from_raw_parts(out_ptr, size).to_vec()
				}
				else {
					res = false;
					Vec::new()
				};

			// The buffer probably needs to be manually freed. I don't think
			// jpeg_destroy_compress() handles that for us.
			libc::free(out_ptr.cast::<c_void>());
			out_ptr = std::ptr::null_mut();
			out_size = 0;

			tmp
		};

	// Release any memory that's left.
	jpeg_destroy_compress(&mut dstinfo);
	jpeg_finish_decompress(&mut srcinfo);
	jpeg_destroy_decompress(&mut srcinfo);

	// Done!
	if res { Some(out) }
	else { None }
}
