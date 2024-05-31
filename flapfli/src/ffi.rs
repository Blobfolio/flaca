/*!
# Flapfli: FFI Image Wrapper.
*/

use std::{
	ffi::c_void,
	ops::Deref,
};



#[derive(Debug)]
/// # Encoded Image.
///
/// This holds a buffer pointer and size for an image allocated in C-land. It
/// exists primarily to enforce cleanup at destruction, but also makes it easy
/// to view the data as a slice.
pub struct EncodedPNG {
	/// # Buffer.
	pub(crate) buf: *mut u8,

	/// # Buffer Size.
	pub(crate) size: usize,
}

impl Deref for EncodedPNG {
	type Target = [u8];

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		if self.is_empty() { &[] }
		else {
			unsafe { std::slice::from_raw_parts(self.buf, self.size) }
		}
	}
}

impl Drop for EncodedPNG {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if ! self.is_empty() {
			unsafe { libc::free(self.buf.cast::<c_void>()); }
			self.buf = std::ptr::null_mut();
		}
	}
}

impl EncodedPNG {
	/// # New.
	pub(crate) const fn new() -> Self {
		Self {
			buf: std::ptr::null_mut(),
			size: 0,
		}
	}

	/// # Is Empty?
	///
	/// Returns true if the instance is empty.
	fn is_empty(&self) -> bool { self.size == 0 || self.buf.is_null() }
}
