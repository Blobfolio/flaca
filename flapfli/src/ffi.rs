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

	#[expect(unsafe_code, reason = "For slice from raw.")]
	#[inline]
	fn deref(&self) -> &Self::Target {
		if self.is_null() { &[] }
		else {
			// Safety: the pointer is non-null.
			unsafe { std::slice::from_raw_parts(self.buf, self.size) }
		}
	}
}

impl Drop for EncodedPNG {
	#[expect(unsafe_code, reason = "For alloc.")]
	fn drop(&mut self) {
		// This pointer is allocated by lodepng, which uses the allocation
		// wrappers defined in this module. To free it, we need to call our
		// method, but only if it actually got allocated.
		if let Some(nn) = NonNull::new(self.buf) {
			// Safety: the pointer is non-null.
			unsafe { flapfli_free(nn); }
			self.buf = std::ptr::null_mut(); // Probably unnecessary?
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

	/// # Is Null?
	///
	/// This is essentially an `is_empty`, returning `true` if the length value
	/// is zero or the buffer pointer is literally null.
	///
	/// (The name was chosen to help avoid conflicts with dereferenced slice
	/// methods.)
	pub(crate) const fn is_null(&self) -> bool { self.size == 0 || self.buf.is_null() }
}



#[expect(unsafe_code, reason = "For alloc.")]
#[expect(clippy::inline_always, reason = "For performance.")]
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
pub(crate) unsafe fn flapfli_allocate(ptr: *mut u8, new_size: NonZeroUsize) -> NonNull<u8> {
	let real_ptr =
		// If we already have an allocation, resize it if needed.
		if let Some(nn) = NonNull::new(ptr) {
			// Get the allocation details.
			// Safety: see the other side.
			let (real_ptr, old_size) = unsafe { size_and_ptr(nn) };

			// Return it as-was if the allocation is already sufficient.
			if old_size >= new_size { return nn; }

			// Safety: old and new should match.
			unsafe {
				realloc(
					real_ptr.as_ptr(),
					layout_for(old_size),
					size_of::<usize>() + new_size.get(),
				)
			}
		}
		// Otherwise get the allocation train up and running!
		else {
			// Safety: the layout is valid.
			unsafe { alloc(layout_for(new_size)) }
		};

	#[expect(clippy::undocumented_unsafe_blocks, reason = "Doesn't fit. Haha.")]
	// Make sure we actually achieved allocation; this shouldn't fail, but
	// might?
	let real_ptr = NonNull::new(real_ptr)
		.unwrap_or_else(#[inline(never)] || handle_alloc_error(unsafe { layout_for(new_size) }));

	// Safety: the layout is aligned to usize.
	unsafe {
		real_ptr.cast::<usize>().write(new_size.get()); // Write the length.
		real_ptr.add(size_of::<usize>())                // Return the rest.
	}
}

#[expect(unsafe_code, reason = "For alloc.")]
#[expect(clippy::inline_always, reason = "For performance.")]
#[inline(always)]
/// # Freedom!
///
/// This method deallocates a pointer previously allocated by
/// `flapfli_allocate`. Refer to that method's documentation for the how and
/// why.
pub(crate) unsafe fn flapfli_free(ptr: NonNull<u8>) {
	// Safety: C sucks; we have to have some trust that they're giving us our
	// own pointers back. Haha.
	unsafe {
		let (ptr, size) = size_and_ptr(ptr);
		dealloc(ptr.as_ptr(), layout_for(size));
	}
}



#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "For FFI.")]
/// # Lodepng-specific Free.
///
/// This override allows lodepng to use `flapfli_free` for pointer
/// deallocation.
unsafe extern "C" fn lodepng_free(ptr: *mut c_void) {
	if let Some(nn) = NonNull::new(ptr.cast()) {
		// Safety: it's non-null at least.
		unsafe { flapfli_free(nn); }
	}
}

#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "For FFI.")]
/// # Lodepng-specific Malloc.
///
/// This override allows lodepng to use `flapfli_allocate` for pointer
/// allocation.
unsafe extern "C" fn lodepng_malloc(size: usize) -> *mut c_void {
	// Safety: see flapfli_allocate.
	unsafe {
		flapfli_allocate(
			std::ptr::null_mut(),
			NonZeroUsize::new(size).unwrap_or(NonZeroUsize::MIN),
		).as_ptr().cast()
	}
}

#[unsafe(no_mangle)]
#[expect(unsafe_code, reason = "For FFI.")]
/// # Lodepng-specific Realloc.
///
/// This override allows lodepng to use `flapfli_allocate` for pointer
/// resizing. For reasons, this will sometimes receive a null pointer, so isn't
/// really any different than `lodepng_malloc`.
unsafe extern "C" fn lodepng_realloc(ptr: *mut c_void, new_size: usize) -> *mut c_void {
	// Safety: see flapfli_allocate.
	unsafe {
		flapfli_allocate(
			ptr.cast(),
			NonZeroUsize::new(new_size).unwrap_or(NonZeroUsize::MIN),
		).as_ptr().cast()
	}
}



#[expect(unsafe_code, reason = "For alloc.")]
#[expect(clippy::inline_always, reason = "For performance.")]
#[inline(always)]
/// # Generate Layout.
///
/// This returns an appropriately sized and aligned layout with room at the
/// beginning to hold our "secret" length information.
const unsafe fn layout_for(size: NonZeroUsize) -> Layout {
	// Safety: size is non-zero.
	unsafe {
		Layout::from_size_align_unchecked(
			size_of::<usize>() + size.get(),
			align_of::<usize>(),
		)
	}
}

#[expect(unsafe_code, reason = "For alloc.")]
#[expect(clippy::inline_always, reason = "For performance.")]
#[inline(always)]
/// # Derive Real Pointer and User Size.
///
/// This method takes the `size`-sized pointer shared with the rest of the
/// crate (and lodepng) and converts it to the "real" one (with the leading
/// length details), returning it and the "size" that can be written to
/// willynilly (i.e. everything minus the extra length-holding portion).
const unsafe fn size_and_ptr(ptr: NonNull<u8>) -> (NonNull<u8>, NonZeroUsize) {
	// Safety: the pointer is non-null and assuming it's ours, is properly
	// aligned and advanced one usize. C sucks. Haha.
	unsafe {
		// Subtract our way to the "real" beginning of the pointer.
		let size_and_data_ptr = ptr.sub(size_of::<usize>());

		// Safety: the size comes from a NonZeroUsize so can be turned back into
		// one.
		let size = NonZeroUsize::new_unchecked(size_and_data_ptr.cast::<usize>().read());

		(size_and_data_ptr, size)
	}
}



#[cfg(test)]
mod tests {
	#[test]
	/// # No Drop Checks.
	///
	/// Prove we aren't missing out by not running drop-in-place or whatever on
	/// usize/byte slices.
	fn t_nodrop() {
		use std::mem::needs_drop;

		assert!(! needs_drop::<[usize]>());
		assert!(! needs_drop::<[u8]>());
	}
}
