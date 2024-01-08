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
	jpeg_common_struct,
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
	ffi::{
		c_int,
		c_uint,
		c_ulong,
	},
	marker::PhantomPinned,
};
use super::ffi::EncodedImage;



// We need a couple more things from jpegtran. Mozjpeg-sys includes the right
// sources but doesn't export these definitions for whatever reason.
extern "C-unwind" {
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

	// Our original image length.
	let src_size = src.len() as c_ulong; // We know this fits.

	// Set up the decompression/compression structs.
	let mut srcinfo = JpegSrcInfo::from(src);
	let mut dstinfo = JpegDstInfo::from(&mut srcinfo);

	unsafe {
		// Load the source file.
		jpeg_mem_src(&mut srcinfo.cinfo, srcinfo.raw.as_ptr(), src_size);

		// Ignore markers.
		jcopy_markers_setup(&mut srcinfo.cinfo, 0);

		// Read the file header to get to the goods.
		jpeg_read_header(&mut srcinfo.cinfo, 1);

		// Read a few more properties into the source struct.
		if jtransform_request_workspace(&mut srcinfo.cinfo, &mut transformoption) == 0 {
			return None;
		}
	}

	// Read source file as DCT coefficients.
	let src_coef_arrays: *mut jvirt_barray_ptr = unsafe {
		jpeg_read_coefficients(&mut srcinfo.cinfo)
	};

	// Initialize destination compression parameters from source values.
	unsafe { jpeg_copy_critical_parameters(&srcinfo.cinfo, &mut dstinfo.cinfo); }

	// Adjust destination parameters if required by transform options, and sync
	// the coefficient arrays.
	let dst_coef_arrays: *mut jvirt_barray_ptr = unsafe {
		jtransform_adjust_parameters(
			&mut srcinfo.cinfo,
			&mut dstinfo.cinfo,
			src_coef_arrays,
			&mut transformoption,
		)
	};

	// Turn on "code optimizing".
	dstinfo.cinfo.optimize_coding = 1;

	// Compress!
	let mut out = EncodedImage::default();
	unsafe {
		// Enable "progressive".
		jpeg_simple_progression(&mut dstinfo.cinfo);

		// And load the destination file.
		jpeg_mem_dest(&mut dstinfo.cinfo, &mut out.buf, &mut out.size);

		// Start the compressor. Note: no data is written here.
		jpeg_write_coefficients(&mut dstinfo.cinfo, dst_coef_arrays);

		// Make sure we aren't copying any markers.
		jcopy_markers_execute(&mut srcinfo.cinfo, &mut dstinfo.cinfo, 0);

		// Execute and write the transformation, if any.
		jtransform_execute_transform(
			&mut srcinfo.cinfo,
			&mut dstinfo.cinfo,
			src_coef_arrays,
			&mut transformoption,
		);
	}

	// Finish it up, and note whether or not it (probably) worked.
	let happy = dstinfo.finish();

	// The decompression will have finished much earlier, but we had to wait
	// to call this deconstructor until now because of all the shared
	// references.
	unsafe { jpeg_finish_decompress(&mut srcinfo.cinfo); }

	// Return it if we got it!
	if happy  && ! out.is_empty() && out.size < src_size { Some(out) }
	else { None }
}



/// # JPEG Source Info.
///
/// This struct is used to parse the source image details and related errors.
/// The abstraction is primarily used to ensure the C-related resources are
/// correctly broken down on drop.
struct JpegSrcInfo<'a> {
	raw: &'a [u8],
	cinfo: jpeg_decompress_struct,
	err: Box<jpeg_error_mgr>,
}

impl<'a> From<&'a [u8]> for JpegSrcInfo<'a> {
	#[allow(unsafe_code)]
	fn from(raw: &'a [u8]) -> Self {
		let mut out = Self {
			raw,
			cinfo: unsafe { std::mem::zeroed() },
			err: new_err(),
		};

		unsafe {
			// Set up the error, then the struct.
			out.cinfo.common.err = std::ptr::addr_of_mut!(*out.err);
			jpeg_create_decompress(&mut out.cinfo);
		}

		out
	}
}

impl<'a> Drop for JpegSrcInfo<'a> {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		unsafe { jpeg_destroy_decompress(&mut self.cinfo); }
	}
}



/// # JPEG Destination Info.
///
/// This struct is used to hold the output-related image details, but not the
/// image itself.
///
/// On the surface, this looks almost exactly like the `JpegSrcInfo` wrapper,
/// but its error is a raw pointer because `mozjpeg` is really weird. Haha.
struct JpegDstInfo {
	cinfo: jpeg_compress_struct,
	err: *mut jpeg_error_mgr,
	_pin: PhantomPinned,
}

impl From<&mut JpegSrcInfo<'_>> for JpegDstInfo {
	#[allow(unsafe_code)]
	fn from(src: &mut JpegSrcInfo<'_>) -> Self {
		let mut out = Self {
			cinfo: unsafe { std::mem::zeroed() },
			err: Box::into_raw(new_err()),
			_pin: PhantomPinned,
		};

		unsafe {
			// Set up the error, then the struct.
			out.cinfo.common.err = std::ptr::addr_of_mut!(*out.err);
			jpeg_create_compress(&mut out.cinfo);

			// Sync the source trace level with the destination.
			src.err.trace_level = (*out.err).trace_level;
		}

		out
	}
}

impl Drop for JpegDstInfo {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		unsafe {
			jpeg_destroy_compress(&mut self.cinfo);

			// The error pointer is no longer accessible.
			let _ = Box::from_raw(self.err);
		}
	}
}

impl JpegDstInfo {
	#[allow(unsafe_code)]
	/// # Finish Compression!
	///
	/// This finishes writing the new image, consuming the details struct in
	/// the process.
	///
	/// A simple `true`/`false` boolean is returned to indicate (likely)
	/// success.
	fn finish(mut self) -> bool {
		unsafe {
			jpeg_finish_compress(&mut self.cinfo);
			0 == (*self.cinfo.common.err).msg_code
		}
	}
}



#[allow(clippy::unnecessary_box_returns, unsafe_code)]
/// # New Unwinding Error.
///
/// Mozjpeg is largely designed to panic anytime there's an error instead of
/// returning helpful status messages or anything like that.
///
/// This initializes a new error struct for de/compression use with handlers
/// set to suppress the messaging and unwind.
///
/// Shout out to the [mozjpeg](https://github.com/ImageOptim/mozjpeg-rust/blob/main/src/errormgr.rs)
/// crate for the inspiration!
fn new_err() -> Box<jpeg_error_mgr> {
	unsafe {
		let mut err = Box::new(std::mem::zeroed());
		jpeg_std_error(&mut err);
		err.error_exit = Some(unwind_error_exit);
		err.emit_message = Some(silence_message);
		err
	}
}

#[cold]
/// # Error Message.
///
/// This is a noop method; no error message is printed.
extern "C-unwind" fn silence_message(_cinfo: &mut jpeg_common_struct, _msg_level: c_int) {}

#[cold]
#[allow(unsafe_code)]
/// # Error Exit.
///
/// Emit an unwinding panic so we can recover somewhat gracefully from mozjpeg
/// errors.
extern "C-unwind" fn unwind_error_exit(_cinfo: &mut jpeg_common_struct) {
	std::panic::resume_unwind(Box::new(()));
}
