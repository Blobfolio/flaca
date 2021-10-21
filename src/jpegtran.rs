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

use crate::FlacaError;
use libc::{
	c_uchar,
	free,
};
use mozjpeg_sys::{
	boolean,
	c_ulong,
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
use std::{
	mem,
	ptr,
	slice,
};



#[allow(clippy::upper_case_acronyms)] // It's C, baby.
const JCOPYOPT_NONE: JCOPY_OPTION = 0;

#[allow(clippy::upper_case_acronyms)] // It's C, baby.
#[allow(non_camel_case_types)]
type JCOPY_OPTION = u32;

// We need a couple more things from jpegtran. Mozjpeg-sys includes the right
// sources but doesn't export the definitions.
extern "C" {
	fn jcopy_markers_setup(srcinfo: j_decompress_ptr, option: JCOPY_OPTION);
}

extern "C" {
	fn jcopy_markers_execute(
		srcinfo: j_decompress_ptr,
		dstinfo: j_compress_ptr,
		option: JCOPY_OPTION,
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
pub unsafe fn jpegtran_mem(data: &[u8]) -> Result<Vec<u8>, FlacaError> {
	let mut transformoption: jpeg_transform_info =
		jpeg_transform_info {
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
			workspace_coef_arrays: ptr::null_mut::<jvirt_barray_ptr>(),
			output_width: 0,
			output_height: 0,
			x_crop_offset: 0,
			y_crop_offset: 0,
			iMCU_sample_width: 0,
			iMCU_sample_height: 0,
		};

	let mut jsrcerr: jpeg_error_mgr = mem::zeroed();
	let mut srcinfo: jpeg_decompress_struct = mem::zeroed();
	srcinfo.common.err = jpeg_std_error(&mut jsrcerr);

	let mut jdsterr: jpeg_error_mgr = mem::zeroed();
	let mut dstinfo: jpeg_compress_struct = mem::zeroed();
	dstinfo.common.err = jpeg_std_error(&mut jdsterr);

	let mut src_coef_arrays: *mut jvirt_barray_ptr = ptr::null_mut::<jvirt_barray_ptr>();
	let mut dst_coef_arrays: *mut jvirt_barray_ptr = ptr::null_mut::<jvirt_barray_ptr>();

	// Initialize the JPEG (de/)compression object with default error handling.
	jpeg_create_decompress(&mut srcinfo);
	jpeg_create_compress(&mut dstinfo);

	// Sync the error trace levels.
	jsrcerr.trace_level = jdsterr.trace_level;

	// Load the source file.
	jpeg_mem_src(&mut srcinfo, data.as_ptr(), data.len() as c_ulong);

	// Ignore markers. This may not be needed, but isn't currently exported by
	// mozjpeg_sys.
	jcopy_markers_setup(&mut srcinfo, JCOPYOPT_NONE);

	// Read the file header to get to the goods.
	jpeg_read_header(&mut srcinfo, true as boolean);

	// Abort if transformation is not possible. We aren't cropping or anything,
	// but this method might still do something with the defaults?
	if jtransform_request_workspace(&mut srcinfo, &mut transformoption) == 0 {
		return Err(FlacaError::ParseFail);
	}

	// Read source file as DCT coefficients.
	src_coef_arrays = jpeg_read_coefficients(&mut srcinfo);

	// Initialize destination compression parameters from source values.
	jpeg_copy_critical_parameters(&srcinfo, &mut dstinfo);

	// Adjust destination parameters if required by transform options, and sync
	// the coefficient arrays.
	dst_coef_arrays = jtransform_adjust_parameters(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);

	// Get an output buffer going.
	let mut outbuffer: *mut c_uchar = ptr::null_mut();
    let mut outsize: c_ulong = 0;

	// Turn on "progressive" and "code optimizing" for the output.
	dstinfo.optimize_coding = true as boolean;
	jpeg_simple_progression(&mut dstinfo);

	// And load the destination file.
	jpeg_mem_dest(&mut dstinfo, &mut outbuffer, &mut outsize);

	// Start the compressor. Note: no data is written here.
	jpeg_write_coefficients(&mut dstinfo, dst_coef_arrays);

	// Make sure we aren't copying any markers.
	jcopy_markers_execute(&mut srcinfo, &mut dstinfo, JCOPYOPT_NONE);

	// Execute and write the transformation, if any.
	jtransform_execute_transform(
		&mut srcinfo,
		&mut dstinfo,
		src_coef_arrays,
		&mut transformoption,
	);

	// Let's get the data!
	jpeg_finish_compress(&mut dstinfo);
	let out: Vec<u8> =
		if outbuffer.is_null() || outsize == 0 { vec![] }
		else {
			let tmp = slice::from_raw_parts(
				outbuffer,
				usize::try_from(outsize).map_err(|_| FlacaError::ParseFail)?
			).to_vec();

			// The buffer probably needs to be manually freed. I don't think
			// jpeg_destroy_compress() handles that for us.
			free(outbuffer.cast::<mozjpeg_sys::c_void>());
			outbuffer = ptr::null_mut();
			outsize = 0;

			tmp
		};

	// Release any memory that's left.
	jpeg_destroy_compress(&mut dstinfo);
	jpeg_finish_decompress(&mut srcinfo);
	jpeg_destroy_decompress(&mut srcinfo);

	// Return the result if any!
	Ok(out)
}
