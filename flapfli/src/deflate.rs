/*!
# Flapfli: Deflate.

This module contains the custom lodepng callback (that uses zopfli), and
supporting components.
*/

use std::{
	cell::RefCell,
	ffi::{
		c_uchar,
		c_uint,
	},
	num::{
		NonZeroUsize,
		NonZeroU32,
	},
	ptr::NonNull,
};
use super::{
	deflate_part,
	ffi::flapfli_allocate,
	lodepng::LodePNGCompressSettings,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliChunk,
	ZopfliState,
};



#[expect(unsafe_code, reason = "Twenty is non-zero.")]
/// # Twenty.
///
/// Safety: twenty is non-zero.
const NZ20: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(20) };

#[expect(unsafe_code, reason = "Sixty is non-zero.")]
/// # Sixty.
///
/// Safety: sixty is Non-Zero.
const NZ60: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(60) };

#[expect(unsafe_code, reason = "`i32::MAX` is non-zero.")]
/// # Max Iterations.
///
/// Safety: `i32::MAX` is non-zero.
const MAX_ITERATIONS: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(i32::MAX as u32) };

/// # Number of Zopfli LZ77 Iterations.
///
/// `Some` values are capped to `i32::MAX`, though anything above a few
/// thousand iterations is madness.
///
/// If `None`, either twenty or sixty iterations will be performed, depending
/// on the file size.
///
/// Note: This value is only (possibly) set (once) during `flaca`'s
/// initialization; it won't change after that.
static mut NUM_ITERATIONS: Option<NonZeroU32> = None;



#[no_mangle]
#[expect(unsafe_code, reason = "For FFI.")]
/// # Custom PNG Deflate.
///
/// This is a custom deflate callback for lodepng. When set, image blocks are
/// compressed using zopfli instead of basic-ass deflate.
///
/// Zopfli is a monster, though, so this is only actually used for the final
/// pass. (Brute force strategizing uses cheaper compression.)
///
/// Following C convention, this returns `0` for success, `1` for sadness.
///
/// ## Safety
///
/// The mutable pointers may or may not initially be null. Allocations are
/// handled on the Rust side, though, and those methods are aware of the fact
/// and will later act (or not act) on these pointer accordingly.
///
/// The `arr`/`insize` values, on the other hand, _should_ definitely be
/// initialized and valid. We can't verify that, but their existence is the
/// whole point of this callback, so it's probably fine…
///
/// Flaca processes images in parallel, but the lodepng/zopfli operations are
/// single-threaded. (All work for a given image happens on a single thread.)
/// This is why we can leverage local statics like `STATE` without fear of
/// access contention.
pub(crate) extern "C" fn flaca_png_deflate(
	out: *mut *mut c_uchar,
	outsize: *mut usize,
	arr: *const c_uchar,
	insize: usize,
	_settings: *const LodePNGCompressSettings,
) -> c_uint {
	thread_local!(
		static STATE: RefCell<Box<ZopfliState>> = RefCell::new(ZopfliState::new())
	);

	// Group the pointer crap to cut down on the number of args being
	// passed around.
	let mut dst = ZopfliOut {
		bp: 0,
		out,
		outsize,
	};

	// Make a proper slice out of the data.
	// Safety: we have to trust that lodepng is giving us accurate information.
	let arr = unsafe { std::slice::from_raw_parts(arr, insize) };

	// Figure out how many iterations to use.
	let numiterations = zopfli_iterations(arr.len());

	// Compress in chunks, à la ZopfliDeflate.
	for chunk in DeflateIter::new(arr) {
		#[cfg(not(debug_assertions))]
		if STATE.with_borrow_mut(|state| deflate_part(
			state,
			numiterations,
			chunk.total_len().get() == arr.len(),
			chunk,
			&mut dst,
		)).is_err() { return 1; };

		#[cfg(debug_assertions)]
		if let Err(e) = STATE.with_borrow_mut(|state| deflate_part(
			state,
			numiterations,
			chunk.total_len().get() == arr.len(),
			chunk,
			&mut dst,
		)) { panic!("{e}"); };
	}

	// All clear!
	0
}

/// # Set Iteration Count.
///
/// Override the default (size-based) number of Zopfli LZ77 iterations with a
/// fixed value.
pub fn set_zopfli_iterations(n: NonZeroU32) {
	#[expect(unsafe_code, reason = "For mut static.")]
	// Safety: this value is only written to once, if that, while the `flaca`
	// binary is parsing the CLI arguments. There won't be any contention for
	// this value.
	unsafe { NUM_ITERATIONS.replace(NonZeroU32::min(n, MAX_ITERATIONS)); }
}


/// # Lodepng Output Pointers.
///
/// This struct serves as a convenience wrapper for the various lodepng/zopfli
/// output pointers, saving us the trouble of passing each of them individually
/// down the rabbit hole.
///
/// This struct also enables us to centralize the convoluted bit-writing
/// methods used to record data, minimizing — as much as possible — the use of
/// `unsafe` everywhere else.
pub(super) struct ZopfliOut {
	/// # Bit Pointer.
	bp: u8,

	/// # Output Buffer.
	out: *mut *mut u8,

	/// # Output (Written) Length.
	outsize: *mut usize,
}

impl ZopfliOut {
	#[expect(unsafe_code, reason = "For alloc.")]
	#[inline]
	/// # Append Data.
	///
	/// This adds a single byte to the output array, re-allocating as
	/// necessary. The `outsize` value is incremented accordingly.
	///
	/// In practice, most data is written bit-by-bite rather than byte-by-byte.
	/// As such, most calls to this method simply write a zero and bit-OR it a
	/// few times afterwards.
	fn append_data(&mut self, value: u8) {
		#[cold]
		/// # Allocate.
		///
		/// Re/allocation is (potentially) necessary whenever `outsize` reaches
		/// a power of two, but since that value represents the length written
		/// rather than the actual capacity, this is often a no-op (after some
		/// checking).
		///
		/// As such, we don't want all this stuff affecting the compiler's
		/// inlining decisions, hence the cold wrapper.
		///
		/// Safety: allocation requires unsafe, but this should be safer than
		/// leaving everything to C!
		unsafe fn alloc_cold(ptr: *mut u8, size: usize) -> NonNull<u8> {
			flapfli_allocate(
				ptr,
				NonZeroUsize::new(size * 2).unwrap_or(NonZeroUsize::MIN),
			)
		}

		// Safety: our allocation wrappers check the pointer is non-null and
		// properly sized.
		unsafe {
			// Dereference the size once to save some sanity.
			let size = *self.outsize;

			// (Re)allocate if size is a power of two, or empty.
			if 0 == (size & size.wrapping_sub(1)) {
				*self.out = alloc_cold(*self.out, size).as_ptr();
			}

			// Write the value and bump the outside length counter.
			(*self.out).add(size).write(value);
			self.outsize.write(size + 1);
		}
	}
}

impl ZopfliOut {
	#[expect(clippy::doc_markdown, reason = "False positive.")]
	#[inline]
	/// # Add Bit.
	///
	/// This adds a single bit to the output array. When the internal `bp`
	/// counter is zero that bit gets added on top of a new zero byte,
	/// otherwise it is ORed on top of the last one.
	pub(crate) fn add_bit(&mut self, bit: u8) {
		if self.bp == 0 { self.append_data(0); }
		#[expect(unsafe_code, reason = "For pointer deref.")]
		// Safety: `append_data` writes a byte to `outsize` and then
		// increments it, so to reach and modify that same position we need
		// to use `outsize - 1` instead.
		unsafe {
			*(*self.out).add(*self.outsize - 1) |= bit << self.bp;
		}
		self.bp = self.bp.wrapping_add(1) & 7;
	}

	/// # Add Multiple Bits.
	///
	/// This method is used to write multiple bits — `length` of them — at
	/// once, shifting on each pass.
	pub(crate) fn add_bits(&mut self, symbol: u32, length: u32) {
		for i in 0..length {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[inline]
	/// # Add Multiple Bits.
	///
	/// Same as `ZopfliOut::add_bits`, but optimized for lengths known at
	/// compile-time.
	///
	/// ## Panics
	///
	/// This will panic at compile-time if `N` is less than two.
	pub(crate) fn add_fixed_bits<const N: u8>(&mut self, symbol: u32) {
		const { assert!(1 < N, "BUG: fixed bits implies more than one!"); }
		for i in const { 0..N } {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[inline]
	/// # Add Type Bits Header.
	///
	/// This writes the three-bit block type header. In practice, there are
	/// only three possible values:
	/// * 0 for uncompressed;
	/// * 1 for fixed;
	/// * 2 for dynamic;
	pub(crate) fn add_header<const BLOCK_BIT: u8>(&mut self, last_block: bool) {
		self.add_bit(u8::from(last_block));
		self.add_bit(const { BLOCK_BIT & 1 });
		self.add_bit(const { (BLOCK_BIT & 2) >> 1 });
	}

	/// # Add Huffman Bits.
	///
	/// Same as `ZopfliOut::add_bits`, but the bits are written in the
	/// reverse order to keep life interesting.
	pub(crate) fn add_huffman_bits(&mut self, symbol: u32, length: u32) {
		// Same as add_bits, except we're doing it backwards.
		for i in (0..length).rev() {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	/// # Add Non-Compressed Block.
	///
	/// As one might suspect, uncompressed blocks are virtually never smaller
	/// than compressed blocks, so this method is included more for
	/// completeness than anything else.
	///
	/// But who knows?
	///
	/// Implementation-wise, this requires no statistical data; it merely
	/// loops through the raw data in chunks of `u16::MAX`, writes some
	/// header/size data, then copies the bytes over.
	pub(crate) fn add_uncompressed_block(
		&mut self,
		last_block: bool,
		chunk: ZopfliChunk<'_>,
	) {
		// We need to proceed u16::MAX bytes at a time.
		let iter = chunk.block().chunks(usize::from(u16::MAX));
		let len = iter.len() - 1;
		for (i, block) in iter.enumerate() {
			let blocksize = block.len();
			let nlen = ! blocksize;
			let really_last_block = i == len;

			// Each chunk gets its own header.
			self.add_header::<0>(last_block && really_last_block);

			// Ignore bits of input up to the next byte boundary.
			self.bp = 0;

			// Some size details.
			self.append_data((blocksize % 256) as u8);
			self.append_data((blocksize.wrapping_div(256) % 256) as u8);
			self.append_data((nlen % 256) as u8);
			self.append_data((nlen.wrapping_div(256) % 256) as u8);

			// And finally the data!
			for byte in block.iter().copied() { self.append_data(byte); }
		}
	}
}



/// # Deflate Chunk Iterator.
///
/// Zopfli processes image data in chunks of (up to) a million bytes, but for
/// some reason it needs to see any previously-seen data on each pass too.
///
/// This iterator thus yields increasingly larger slices of `arr`, until
/// eventually the whole thing is returned. The internal `pos` value tracks the
/// start of the "active" portion.
///
/// See `ZopfliChunk` for more information. Haha.
struct DeflateIter<'a> {
	/// # Data.
	arr: &'a [u8],

	/// # Window Start.
	pos: usize,
}

impl<'a> Iterator for DeflateIter<'a> {
	type Item = ZopfliChunk<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		if self.pos < self.arr.len() {
			let pos = self.pos;
			let chunk = self.arr.get(..pos + ZOPFLI_MASTER_BLOCK_SIZE).unwrap_or(self.arr);
			self.pos = chunk.len();
			ZopfliChunk::new(chunk, pos).ok()
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for DeflateIter<'a> {
	fn len(&self) -> usize {
		(self.arr.len() - self.pos).div_ceil(ZOPFLI_MASTER_BLOCK_SIZE)
	}
}

impl<'a> DeflateIter<'a> {
	/// # New.
	const fn new(arr: &'a [u8]) -> Self {
		Self { arr, pos: 0 }
	}
}



#[expect(unsafe_code, reason = "Read mut static.")]
/// # Number of Zopfli LZ77 Iterations.
///
/// This either returns the user's fixed preference, or a size-based fallback.
fn zopfli_iterations(len: usize) -> NonZeroU32 {
	// Safety: this value is only ever set during flaca initialization; there
	// is no thread contention.
	unsafe { NUM_ITERATIONS }.unwrap_or(
		if len < 200_000 { NZ60 } else { NZ20 }
	)
}
