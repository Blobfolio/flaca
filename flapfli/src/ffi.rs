/*!
# Flaca: FFI

This module contains some general-purpose FFI helpers.
*/

use std::{
	ffi::{
		c_uchar,
		c_ulong,
		c_void,
	},
	ops::Deref,
};



#[derive(Debug)]
/// # Encoded Image.
///
/// This holds a buffer pointer and size for an image allocated in C-land. It
/// exists primarily to enforce cleanup at destruction, but also makes it easy
/// to view the data as a slice.
pub struct EncodedImage<T> {
	/// # Buffer.
	pub buf: *mut c_uchar,

	/// # Buffer Size.
	pub size: T,
}

macro_rules! default {
	($($ty:ty),+) => ($(
		impl Default for EncodedImage<$ty> {
			#[inline]
			fn default() -> Self {
				Self { buf: std::ptr::null_mut(), size: 0 }
			}
		}
	)+);
}
default!(c_ulong, usize);

impl Deref for EncodedImage<c_ulong> {
	type Target = [u8];

	#[allow(clippy::cast_possible_truncation, unsafe_code)]
	fn deref(&self) -> &Self::Target {
		if self.is_empty() { &[] }
		else {
			unsafe { std::slice::from_raw_parts(self.buf, self.size as usize) }
		}
	}
}

impl Deref for EncodedImage<usize> {
	type Target = [u8];

	#[allow(unsafe_code)]
	fn deref(&self) -> &Self::Target {
		if self.is_empty() { &[] }
		else {
			unsafe { std::slice::from_raw_parts(self.buf, self.size) }
		}
	}
}

impl<T> Drop for EncodedImage<T> {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if ! self.buf.is_null() {
			unsafe { libc::free(self.buf.cast::<c_void>()); }
			self.buf = std::ptr::null_mut();
		}
	}
}

macro_rules! is_empty {
	($($ty:ty),+) => ($(
		impl EncodedImage<$ty> {
			#[must_use]
			/// # Is Empty?
			///
			/// Returns true if the instance is empty.
			fn is_empty(&self) -> bool { self.size == 0 || self.buf.is_null() }
		}
	)+);
}
is_empty!(c_ulong, usize);
