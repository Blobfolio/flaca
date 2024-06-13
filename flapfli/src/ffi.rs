/*!
# Flapfli: FFI Image Wrapper.

This module contains custom allocation wrappers for `lodepng`, allowing Rust
to (more or less) manage the memory.
*/

use std::{
	alloc::{
		alloc,
		dealloc,
		handle_alloc_error,
		Layout,
		realloc,
	},
	ffi::c_void,
	ops::Deref,
	ptr::NonNull,
};



/// # Size of Usize.
const USIZE_SIZE: usize = std::mem::size_of::<usize>();



#[derive(Debug)]
/// # Encoded Image.
///
/// This is a convenience wrapper for an image encoded by `lodepng`, allowing
/// for easy slice dereferencing and automatic drop cleanup.
///
/// Note the initial state is null/empty.
pub struct EncodedPNG {
	/// # Buffer.
	pub(crate) buf: *mut u8,

	/// # Buffer Size.
	pub(crate) size: usize,
}

impl Deref for EncodedPNG {
	type Target = [u8];

	#[allow(unsafe_code)]
	#[inline]
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
		unsafe { flapfli_free(self.buf); }
		self.buf = std::ptr::null_mut();
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



#[allow(unsafe_code, clippy::cast_ptr_alignment, clippy::inline_always)]
#[inline(always)]
/// # (Re)Allocate!
///
/// Allocate (or reallocate) and return a new pointer for `size` bytes that can
/// be used by the crate or `lodepng` or both.
///
/// Since C can't be trusted to keep track of allocation sizes, we use the same
/// trick the [`libdeflater`](https://github.com/adamkewley/libdeflater/blob/master/src/malloc_wrapper.rs) crate does;
/// we over-allocate by `size_of::<usize>()` bytes, use that extra space to
/// hold the length details, and return the rest so the caller gets what it
/// expects.
///
/// This still requires a lot of unsafe, but at least it lives on this side of
/// the FFI divide!
pub(crate) unsafe fn flapfli_allocate(ptr: *mut u8, new_size: usize) -> *mut u8 {
	let real_ptr =
		// If null, allocate it fresh.
		if ptr.is_null() {
			let layout = layout_for(new_size);
			NonNull::new(alloc(layout))
				.unwrap_or_else(|| handle_alloc_error(layout))
				.as_ptr()
		}
		// Otherwise resize!
		else {
			let (real_ptr, old_size) = size_and_ptr(ptr);
			realloc(real_ptr, layout_for(old_size), new_size + USIZE_SIZE)
		};

	// Safety: the layout is aligned to usize.
	real_ptr.cast::<usize>().write(new_size); // Write the length.
	real_ptr.add(USIZE_SIZE)                  // Return the rest.
}

#[allow(unsafe_code, clippy::inline_always)]
#[inline(always)]
/// # (Re)Allocate!
///
/// Allocate (or reallocate) and return a new pointer for `size` bytes that can
/// be used by the crate or C or both.
///
/// The trick — courtesy of the [`libdeflater`](https://github.com/adamkewley/libdeflater/blob/master/src/malloc_wrapper.rs) crate —
/// is we over-allocate by `size_of::<usize>()`, using that extra space to hold
/// the length so that later on, we can de- or re-allocate correctly.
///
/// This still requires a lot of unsafe, but at least that unsafe lives here!
pub(crate) unsafe fn flapfli_free(ptr: *mut u8) {
	if ! ptr.is_null() {
		let (ptr, size) = size_and_ptr(ptr);
		dealloc(ptr, layout_for(size));
	}
}



#[no_mangle]
#[allow(unsafe_code)]
/// # Free Willy.
unsafe extern "C" fn lodepng_free(ptr: *mut c_void) { flapfli_free(ptr.cast()); }

#[no_mangle]
#[allow(unsafe_code)]
/// # Lodepng-specific Malloc.
///
/// This is the same as ours, but casts to `c_void` for the ABI.
unsafe extern "C" fn lodepng_malloc(size: usize) -> *mut c_void {
	flapfli_allocate(std::ptr::null_mut(), size).cast()
}

#[no_mangle]
#[allow(unsafe_code)]
/// # Re-allocate!
unsafe extern "C" fn lodepng_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
	flapfli_allocate(ptr.cast(), new_size).cast()
}



#[allow(unsafe_code, clippy::inline_always)]
#[inline(always)]
/// # Generate Layout.
///
/// This returns an appropriately sized and aligned layout with room at the
/// beginning to hold our "secret" length information.
const unsafe fn layout_for(size: usize) -> Layout {
	Layout::from_size_align_unchecked(USIZE_SIZE + size, std::mem::align_of::<usize>())
}

#[allow(unsafe_code, clippy::cast_ptr_alignment, clippy::inline_always)]
#[inline(always)]
/// # Derive Real Pointer and User Size.
///
/// This method takes the `size`-sized pointer shared with the rest of the
/// crate (and `lodepng`) and converts it to the "real" one containing the
/// extra length information, returning it along with said length.
const unsafe fn size_and_ptr(ptr: *mut u8) -> (*mut u8, usize) {
	let size_and_data_ptr = ptr.sub(USIZE_SIZE);
	let size = *(size_and_data_ptr as *const usize);
	(size_and_data_ptr, size)
}
