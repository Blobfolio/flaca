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
use std::ffi::{
	c_uint,
	c_ulong,
};
use super::ffi::EncodedImage;



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



#[allow(unsafe_code)]
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
pub(super) fn optimize(src: &[u8]) -> Option<EncodedImage<c_ulong>> {
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

	let mut meta = InOut::default();
	let src_size = src.len() as c_ulong; // We know this fits.

	unsafe {
		// Load the source file.
		jpeg_mem_src(&mut meta.src, src.as_ptr(), src_size);

		// Ignore markers.
		jcopy_markers_setup(&mut meta.src, 0);

		// Read the file header to get to the goods.
		jpeg_read_header(&mut meta.src, 1);

		// Read a few more properties into the source struct.
		if jtransform_request_workspace(&mut meta.src, &mut transformoption) == 0 {
			return None;
		}
	}

	// Read source file as DCT coefficients.
	let src_coef_arrays: *mut jvirt_barray_ptr = unsafe {
		jpeg_read_coefficients(&mut meta.src)
	};

	// Initialize destination compression parameters from source values.
	unsafe { jpeg_copy_critical_parameters(&meta.src, &mut meta.dst); }

	// Adjust destination parameters if required by transform options, and sync
	// the coefficient arrays.
	let dst_coef_arrays: *mut jvirt_barray_ptr = unsafe {
		jtransform_adjust_parameters(
			&mut meta.src,
			&mut meta.dst,
			src_coef_arrays,
			&mut transformoption,
		)
	};

	// Turn on "code optimizing".
	meta.dst.optimize_coding = 1;
	let mut out = EncodedImage::default();
	unsafe {
		// Enable "progressive".
		jpeg_simple_progression(&mut meta.dst);

		// And load the destination file.
		jpeg_mem_dest(&mut meta.dst, &mut out.buf, &mut out.size);

		// Start the compressor. Note: no data is written here.
		jpeg_write_coefficients(&mut meta.dst, dst_coef_arrays);

		// Make sure we aren't copying any markers.
		jcopy_markers_execute(&mut meta.src, &mut meta.dst, 0);

		// Execute and write the transformation, if any.
		jtransform_execute_transform(
			&mut meta.src,
			&mut meta.dst,
			src_coef_arrays,
			&mut transformoption,
		);
	}

	// Return it if we got it!
	if meta.build() && ! out.is_empty() && out.size < src_size {
		Some(out)
	}
	else { None }
}



/// # Source and Destination Data.
///
/// This wrapper struct exists to help ensure C memory is freed correctly on
/// exit.
struct InOut {
	src_err: jpeg_error_mgr,
	src: jpeg_decompress_struct,
	dst_err: jpeg_error_mgr,
	dst: jpeg_compress_struct,
	built: bool,
}

impl Default for InOut {
	#[allow(unsafe_code)]
	fn default() -> Self {
		let mut out = Self {
			src_err: unsafe { std::mem::zeroed() },
			src: unsafe { std::mem::zeroed() },
			dst_err: unsafe { std::mem::zeroed() },
			dst: unsafe { std::mem::zeroed() },
			built: false,
		};

		// Initialize the memory.
		unsafe {
			out.src.common.err = jpeg_std_error(&mut out.src_err);
			out.dst.common.err = jpeg_std_error(&mut out.dst_err);
			jpeg_create_decompress(&mut out.src);
			jpeg_create_compress(&mut out.dst);
		}

		// The trace levels should already match, but just in case…
		out.src_err.trace_level = out.dst_err.trace_level;

		// Done!
		out
	}
}

impl InOut {
	#[allow(unsafe_code)]
	/// # Finish Compression.
	fn build(&mut self) -> bool {
		// Only build once.
		if self.built { false }
		else {
			unsafe { jpeg_finish_compress(&mut self.dst) };
			self.built = true;
			0 == unsafe { (*self.dst.common.err).msg_code }
		}
	}
}

impl Drop for InOut {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		self.build();
		unsafe {
			jpeg_destroy_compress(&mut self.dst);
			jpeg_finish_decompress(&mut self.src);
			jpeg_destroy_decompress(&mut self.src);
		}
	}
}
