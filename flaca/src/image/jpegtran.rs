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
	jcopy_markers_execute,
	jcopy_markers_setup,
	JCOPY_OPTION_JCOPYOPT_NONE,
	JCROP_CODE_JCROP_UNSET,
	jpeg_common_struct,
	jpeg_compress_struct,
	jpeg_copy_critical_parameters,
	jpeg_create_decompress,
	jpeg_CreateCompress,
	jpeg_decompress_struct,
	jpeg_destroy_compress,
	jpeg_destroy_decompress,
	jpeg_error_mgr,
	jpeg_finish_compress,
	jpeg_finish_decompress,
	JPEG_LIB_VERSION,
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
		c_uchar,
		c_ulong,
		c_void,
	},
	marker::PhantomPinned,
	ops::Deref,
	ptr::NonNull,
};



#[derive(Debug)]
/// # Encoded Image.
///
/// This holds a buffer pointer and size for an image allocated in C-land. It
/// exists primarily to enforce cleanup at destruction, but also makes it easy
/// to view the data as a slice.
pub(super) struct EncodedJPEG {
	/// # Buffer.
	buf: *mut c_uchar,

	/// # Buffer Size.
	size: c_ulong,
}

impl Deref for EncodedJPEG {
	type Target = [u8];

	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	#[expect(unsafe_code, reason = "For slice from raw.")]
	fn deref(&self) -> &Self::Target {
		if self.is_null() { &[] }
		else {
			// Safety: the pointer is non-null.
			unsafe { std::slice::from_raw_parts(self.buf, self.size as usize) }
		}
	}
}

impl Drop for EncodedJPEG {
	#[expect(unsafe_code, reason = "For FFI.")]
	fn drop(&mut self) {
		if ! self.buf.is_null() {
			// Safety: the pointer is non-null and was created by C, so if
			// anybody knows what to do with it, C should!
			unsafe { libc::free(self.buf.cast::<c_void>()); }
			self.buf = std::ptr::null_mut(); // Probably unnecessary?
		}
	}
}

impl EncodedJPEG {
	/// # New.
	const fn new() -> Self {
		Self {
			buf: std::ptr::null_mut(),
			size: 0,
		}
	}

	/// # Is Null?
	///
	/// This is essentially an `is_empty`, returning `true` if the length value
	/// is zero or the buffer pointer is literally null.
	///
	/// (The name was chosen to help avoid conflicts with dereferenced slice
	/// methods.)
	const fn is_null(&self) -> bool { self.size == 0 || self.buf.is_null() }
}



#[expect(clippy::inline_always, reason = "For performance.")]
#[expect(unsafe_code, reason = "For FFI.")]
#[inline(always)]
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
pub(super) fn optimize(src: &[u8]) -> Option<EncodedJPEG> {
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

	// Safety: these are FFI calls…
	unsafe {
		// Load the source file.
		jpeg_mem_src(&mut srcinfo.cinfo, srcinfo.raw.as_ptr(), src_size);

		// Ignore markers.
		jcopy_markers_setup(&mut srcinfo.cinfo, JCOPY_OPTION_JCOPYOPT_NONE);

		// Read the file header to get to the goods.
		jpeg_read_header(&mut srcinfo.cinfo, 1);

		// Read a few more properties into the source struct.
		if jtransform_request_workspace(&mut srcinfo.cinfo, &mut transformoption) == 0 {
			return None;
		}
	}

	// Read source file as DCT coefficients.
	// Safety: this is an FFI call…
	let src_coef_arrays: *mut jvirt_barray_ptr = unsafe {
		jpeg_read_coefficients(&mut srcinfo.cinfo)
	};

	// Initialize destination compression parameters from source values.
	// Safety: this is an FFI call…
	unsafe { jpeg_copy_critical_parameters(&srcinfo.cinfo, &mut dstinfo.cinfo); }

	// Adjust destination parameters if required by transform options, and sync
	// the coefficient arrays.
	// Safety: this is an FFI call…
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
	let mut out = EncodedJPEG::new();
	// Safety: these are FFI calls…
	unsafe {
		// Enable "progressive".
		jpeg_simple_progression(&mut dstinfo.cinfo);

		// And load the destination file.
		jpeg_mem_dest(&mut dstinfo.cinfo, &mut out.buf, &mut out.size);

		// Start the compressor. Note: no data is written here.
		jpeg_write_coefficients(&mut dstinfo.cinfo, dst_coef_arrays);

		// Make sure we aren't copying any markers.
		jcopy_markers_execute(&mut srcinfo.cinfo, &mut dstinfo.cinfo, JCOPY_OPTION_JCOPYOPT_NONE);

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
	// Safety: this is an FFI call…
	unsafe { jpeg_finish_decompress(&mut srcinfo.cinfo); }

	// Return it if we got it!
	if happy  && ! out.is_null() && out.size < src_size { Some(out) }
	else { None }
}



/// # JPEG Source Info.
///
/// This struct is used to parse the source image details and related errors.
/// The abstraction is primarily used to ensure the C-related resources are
/// correctly broken down on drop.
struct JpegSrcInfo<'a> {
	/// # Source Data.
	raw: &'a [u8],

	/// # Decompressor.
	cinfo: jpeg_decompress_struct,

	/// # Error Instance.
	err: Box<jpeg_error_mgr>,
}

impl<'a> From<&'a [u8]> for JpegSrcInfo<'a> {
	#[expect(unsafe_code, reason = "For FFI.")]
	fn from(raw: &'a [u8]) -> Self {
		let mut out = Self {
			raw,
			// Safety: the subsequent FFI call expects zeroed memory.
			cinfo: unsafe { std::mem::zeroed() },
			err: new_err(),
		};

		// Safety: and here is that FFI call…
		unsafe {
			// Set up the error, then the struct.
			out.cinfo.common.err = std::ptr::addr_of_mut!(*out.err);
			jpeg_create_decompress(&mut out.cinfo);
		}

		out
	}
}

impl Drop for JpegSrcInfo<'_> {
	#[expect(unsafe_code, reason = "For FFI.")]
	fn drop(&mut self) {
		// Safety: mozjpeg handles deallocation itself.
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
	/// # Compressor.
	cinfo: jpeg_compress_struct,

	/// # Error Instance.
	err: NonNull<jpeg_error_mgr>,

	/// # Pinned Data.
	_pin: PhantomPinned,
}

impl From<&mut JpegSrcInfo<'_>> for JpegDstInfo {
	#[expect(unsafe_code, reason = "For FFI.")]
	fn from(src: &mut JpegSrcInfo<'_>) -> Self {
		let mut out = Self {
			// Safety: the subsequent FFI call requires zeroed memory.
			cinfo: unsafe { std::mem::zeroed() },
			// Safety: boxes point somewhere!
			err: unsafe { NonNull::new_unchecked(Box::into_raw(new_err())) },
			_pin: PhantomPinned,
		};

		// Safety: these are FFI calls…
		unsafe {
			// Set up the error, then the struct.
			out.cinfo.common.err = out.err.as_ptr();
			jpeg_CreateCompress(&mut out.cinfo, JPEG_LIB_VERSION, size_of_val(&out.cinfo));

			// Note: depending on the compiler/flags, JPEG compression can
			// segfault if this isn't explicitly made null. Not sure why it
			// isn't an always/never behavior…
			out.cinfo.common.progress = std::ptr::null_mut();

			// Sync the source trace level with the destination.
			src.err.trace_level = out.err.as_ref().trace_level;
		}

		out
	}
}

impl Drop for JpegDstInfo {
	#[expect(unsafe_code, reason = "For FFI.")]
	fn drop(&mut self) {
		// Safety: mozjpeg handles deallocation itself.
		unsafe {
			jpeg_destroy_compress(&mut self.cinfo);
			let _ = Box::from_raw(self.err.as_ptr());
		}
	}
}

impl JpegDstInfo {
	#[expect(unsafe_code, reason = "For FFI.")]
	/// # Finish Compression!
	///
	/// This finishes writing the new image, consuming the details struct in
	/// the process.
	///
	/// A simple `true`/`false` boolean is returned to indicate (likely)
	/// success.
	fn finish(mut self) -> bool {
		// Safety: mozjpeg handles deallocation itself.
		unsafe {
			jpeg_finish_compress(&mut self.cinfo);
			0 == (*self.cinfo.common.err).msg_code
		}
	}
}



#[expect(clippy::unnecessary_box_returns, reason = "We want a box.")]
#[expect(unsafe_code, reason = "For FFI.")]
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
	// Safety: the FFI call requires zeroed memory to start from.
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
/// # Error Exit.
///
/// Emit an unwinding panic so we can recover somewhat gracefully from mozjpeg
/// errors.
extern "C-unwind" fn unwind_error_exit(_cinfo: &mut jpeg_common_struct) {
	std::panic::resume_unwind(Box::new(()));
}
