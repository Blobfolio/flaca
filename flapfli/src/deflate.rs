/*!
# Flapfli: Deflate
*/

use std::{
	cell::RefCell,
	ffi::{
		c_uchar,
		c_uint,
	},
	num::NonZeroUsize,
	sync::atomic::Ordering::Relaxed,
};
use super::{
	deflate_part,
	ffi::flapfli_allocate,
	lodepng::LodePNGCompressSettings,
	reset_dynamic_length_cache,
	ZOPFLI_ITERATIONS,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliChunk,
	ZopfliState,
};



#[no_mangle]
#[allow(unsafe_code)]
/// # Custom PNG Deflate.
///
/// This tells lodepng to use zopfli for encoding.
pub(crate) extern "C" fn flaca_png_deflate(
	out: *mut *mut c_uchar,
	outsize: *mut usize,
	arr: *const c_uchar,
	insize: usize,
	_settings: *const LodePNGCompressSettings,
) -> c_uint {
	thread_local!(
		static STATES: RefCell<Box<ZopfliState>> = RefCell::new(ZopfliState::new())
	);

	// Group the pointer crap to cut down on the number of args being
	// passed around.
	let mut dst = ZopfliOut {
		bp: 0,
		out,
		outsize,
	};

	// Make a proper slice out of the data.
	let arr = unsafe { std::slice::from_raw_parts(arr, insize) };

	// Figure out how many iterations to use.
	let mut numiterations = ZOPFLI_ITERATIONS.load(Relaxed);
	if numiterations <= 0 {
		numiterations = if arr.len() < 200_000 { 60 } else { 20 };
	}

	// The RLE cache lives for the duration of the image; let's go ahead and
	// reset that now.
	reset_dynamic_length_cache();

	// Compress in chunks, à la ZopfliDeflate.
	for chunk in DeflateIter::new(arr) {
		#[cfg(not(debug_assertions))]
		if STATES.with_borrow_mut(|state| deflate_part(
			state,
			numiterations,
			chunk.total_len().get() == arr.len(),
			chunk,
			&mut dst,
		)).is_err() { return 1; };

		#[cfg(debug_assertions)]
		if let Err(e) = STATES.with_borrow_mut(|state| deflate_part(
			state,
			numiterations,
			chunk.total_len().get() == arr.len(),
			chunk,
			&mut dst,
		)) { panic!("{e}"); };
	}

	// Errors panic, so if we're here everything must be fine.
	0
}



/// # Lodepng Output Pointers.
///
/// This struct provides a wrapper around the lingering bit-writing zopfli C
/// methods, saving us the trouble of having to pass down three different
/// pointers (and using a bunch of unsafe blocks) just to get the data saved.
pub(super) struct ZopfliOut {
	bp: u8,
	out: *mut *mut u8,
	outsize: *mut usize,
}

impl ZopfliOut {
	#[allow(unsafe_code)]
	#[inline]
	/// # Append Data.
	fn append_data(&mut self, value: u8) {
		#[cold]
		/// # Allocate.
		unsafe fn alloc_cold(ptr: *mut u8, size: usize) -> *mut u8 {
			flapfli_allocate(
				ptr,
				NonZeroUsize::new(size * 2).unwrap_or(NonZeroUsize::MIN),
			)
		}

		unsafe {
			// Dereferencing this size gets annoying quick! Haha.
			let size = *self.outsize;

			// (Re)allocate if size is a power of two, or empty.
			if 0 == (size & size.wrapping_sub(1)) {
				*self.out = alloc_cold(*self.out, size);
			}

			(*self.out).add(size).write(value);
			self.outsize.write(size + 1);
		}
	}
}

impl ZopfliOut {
	#[inline]
	/// # Add Bit.
	pub(crate) fn add_bit(&mut self, bit: u8) {
		if self.bp == 0 { self.append_data(0); }
		#[allow(unsafe_code)]
		unsafe {
			// Safety: `append_data` writes a byte to `outsize` and then
			// increments it, so to reach and modify that same position we need
			// to use `outsize - 1` instead.
			*(*self.out).add(*self.outsize - 1) |= bit << self.bp;
		}
		self.bp = self.bp.wrapping_add(1) & 7;
	}

	/// # Add Multiple Bits.
	pub(crate) fn add_bits(&mut self, symbol: u32, length: u32) {
		for i in 0..length {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[inline]
	/// # Add Multiple Bits.
	///
	/// Same as `ZopfliOut::add_bits`, but with lengths known at compile time.
	pub(crate) fn add_fixed_bits<const N: u8>(&mut self, symbol: u32) {
		const { assert!(1 < N); }
		for i in const { 0..N } {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[inline]
	/// # Add Type Bits Header.
	pub(crate) fn add_header<const BLOCK_BIT: u8>(&mut self, last_block: bool) {
		self.add_bit(u8::from(last_block));
		self.add_bit(const { BLOCK_BIT & 1 });
		self.add_bit(const { (BLOCK_BIT & 2) >> 1 });
	}

	/// # Add Huffman Bits.
	pub(crate) fn add_huffman_bits(&mut self, symbol: u32, length: u32) {
		// Same as add_bits, except we're doing it backwards.
		for i in (0..length).rev() {
			let bit = (symbol >> i) & 1;
			self.add_bit(bit as u8);
		}
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Add Non-Compressed Block.
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

			self.append_data((blocksize % 256) as u8);
			self.append_data((blocksize.wrapping_div(256) % 256) as u8);
			self.append_data((nlen % 256) as u8);
			self.append_data((nlen.wrapping_div(256) % 256) as u8);

			for bit in block.iter().copied() { self.append_data(bit); }
		}
	}
}



/// # Deflate Chunk Iterator.
///
/// This yields slices of `arr` from the beginning, increasing the length each
/// time by `ZOPFLI_MASTER_BLOCK_SIZE`.
struct DeflateIter<'a> {
	arr: &'a [u8],
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
