/*!
# Flapfli: FFI Image Wrapper.

This module contains custom allocation wrappers for lodepng, allowing Rust
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
	num::NonZeroUsize,
	ops::Deref,
	ptr::NonNull,
};



#[derive(Debug)]
/// # Encoded Image.
///
/// This is a convenience wrapper for an image encoded by lodepng, allowing
/// for easy slice dereferencing and automatic drop cleanup.
///
/// Note the initial state will be null/empty.
///
/// Allocations are handled by Rust, at least, and are aware of that fact so
/// will act (or not act) on the pointers accordingly.
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
		if self.is_null() { &[] }
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

	/// # Is Null?
	///
	/// This is essentially an `is_empty`, returning `true` if the length value
	/// is zero or the buffer pointer is literally null.
	///
	/// (The name was chosen to help avoid conflicts with dereferenced slice
	/// methods.)
	pub(crate) fn is_null(&self) -> bool { self.size == 0 || self.buf.is_null() }
}



#[allow(unsafe_code, clippy::cast_ptr_alignment, clippy::inline_always)]
#[inline(always)]
/// # (Re)Allocate!
///
/// Allocate (or reallocate) and return a new pointer for `size` bytes that can
/// be used by the crate or lodepng or both.
///
/// Since C can't be trusted to keep track of allocation sizes, we use the same
/// trick the [`libdeflater`](https://github.com/adamkewley/libdeflater/blob/master/src/malloc_wrapper.rs) crate does;
/// we over-allocate by `size_of::<usize>()` bytes, using that extra space to
/// hold the length details.
///
/// The caller then gets `ptr.add(size_of::<usize>())` sized as they expect it
/// to be, and when that pointer is returned to us, we can subtract the same
/// amount to find the length. Rinse and repeat.
///
/// This still requires a lot of unsafe, but at least it lives on this side of
/// the FFI divide!
pub(crate) unsafe fn flapfli_allocate(ptr: *mut u8, new_size: NonZeroUsize) -> *mut u8 {
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
			// Return it as-was if the allocation is already sufficient.
			if old_size >= new_size { return ptr; }
			realloc(real_ptr, layout_for(old_size), size_of::<usize>() + new_size.get())
		};

	// Safety: the layout is aligned to usize.
	real_ptr.cast::<usize>().write(new_size.get()); // Write the length.
	real_ptr.add(size_of::<usize>())                // Return the rest.
}

#[allow(unsafe_code, clippy::inline_always)]
#[inline(always)]
/// # Freedom!
///
/// This method deallocates a pointer previously allocated by
/// `flapfli_allocate`. Refer to that method's documentation for the how and
/// why.
pub(crate) unsafe fn flapfli_free(ptr: *mut u8) {
	if ! ptr.is_null() {
		let (ptr, size) = size_and_ptr(ptr);
		dealloc(ptr, layout_for(size));
	}
}



#[no_mangle]
#[allow(unsafe_code)]
/// # Lodepng-specific Free.
///
/// This override allows lodepng to use `flapfli_free` for pointer
/// deallocation.
unsafe extern "C" fn lodepng_free(ptr: *mut c_void) { flapfli_free(ptr.cast()); }

#[no_mangle]
#[allow(unsafe_code)]
/// # Lodepng-specific Malloc.
///
/// This override allows lodepng to use `flapfli_allocate` for pointer
/// allocation.
unsafe extern "C" fn lodepng_malloc(size: usize) -> *mut c_void {
	flapfli_allocate(
		std::ptr::null_mut(),
		NonZeroUsize::new(size).unwrap_or(NonZeroUsize::MIN),
	).cast()
}

#[no_mangle]
#[allow(unsafe_code)]
/// # Lodepng-specific Realloc.
///
/// This override allows lodepng to use `flapfli_allocate` for pointer
/// resizing.
unsafe extern "C" fn lodepng_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
	flapfli_allocate(
		ptr.cast(),
		NonZeroUsize::new(new_size).unwrap_or(NonZeroUsize::MIN),
	).cast()
}



#[allow(unsafe_code, clippy::inline_always)]
#[inline(always)]
/// # Generate Layout.
///
/// This returns an appropriately sized and aligned layout with room at the
/// beginning to hold our "secret" length information.
const unsafe fn layout_for(size: NonZeroUsize) -> Layout {
	Layout::from_size_align_unchecked(size_of::<usize>() + size.get(), align_of::<usize>())
}

#[allow(unsafe_code, clippy::cast_ptr_alignment, clippy::inline_always)]
#[inline(always)]
/// # Derive Real Pointer and User Size.
///
/// This method takes the `size`-sized pointer shared with the rest of the
/// crate (and lodepng) and converts it to the "real" one (with the leading
/// length details), returning it and the logical size (i.e. minus eight bytes
/// or whatever).
const unsafe fn size_and_ptr(ptr: *mut u8) -> (*mut u8, NonZeroUsize) {
	let size_and_data_ptr = ptr.sub(size_of::<usize>());
	// Safety: the size is written from a NonZeroUsize.
	let size = NonZeroUsize::new_unchecked(*(size_and_data_ptr as *const usize));
	(size_and_data_ptr, size)
}
