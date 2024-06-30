/*!
# Flapfli: Slice Chunks.
*/

use std::num::NonZeroUsize;
use super::{
	zopfli_error,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZOPFLI_WINDOW_SIZE,
	ZopfliError,
	ZopfliRange,
};



#[derive(Debug, Clone, Copy)]
/// # Deflate Chunk.
///
/// The deflate/zopfli process is weird. The data is sliced in
/// `ZOPFLI_MASTER_BLOCK_SIZE` windows, kinda, but the previous data is
/// included for the ride because they're sometimes relevant for hashing.
///
/// Similar to `ZopfliRange`, this struct mainly exists to help enforce the
/// logical constraints so we don't have to repeat sanity checks every five
/// seconds.
pub(crate) struct ZopfliChunk<'a> {
	arr: &'a [u8],
	from: usize,
}

impl<'a> ZopfliChunk<'a> {
	/// # New.
	///
	/// Define a new chunk with the given source and starting point.
	///
	/// ## Errors.
	///
	/// This will return an error if the slice is empty, `from` is out of
	/// range, or the length from `from` is greater than
	/// `ZOPFLI_MASTER_BLOCK_SIZE`.
	pub(crate) const fn new(arr: &'a [u8], from: usize) -> Result<Self, ZopfliError> {
		if from < arr.len() && arr.len() - from <= ZOPFLI_MASTER_BLOCK_SIZE {
			Ok(Self { arr, from })
		}
		else { Err(zopfli_error!()) }
	}

	/// # Reslice.
	///
	/// Return a new instance capped to the range, or an error if the range is
	/// out of bounds.
	pub(crate) fn reslice(&self, start: usize, end: usize) -> Result<Self, ZopfliError> {
		if start < end && end - start <= ZOPFLI_MASTER_BLOCK_SIZE && end <= self.arr.len() {
			let arr = &self.arr[..end];
			Ok(Self { arr, from: start })
		}
		else { Err(zopfli_error!()) }
	}

	/// # Reslice to Range.
	///
	/// Return a new instance capped to the range, or an error if the range is
	/// out of bounds.
	///
	/// (The range's `start` becomes the chunk's `from`, and if `end` is less
	/// than the total length, the array is truncated.)
	pub(crate) fn reslice_rng(&self, rng: ZopfliRange) -> Result<Self, ZopfliError> {
		let arr = self.arr.get(..rng.end()).ok_or(zopfli_error!())?;
		Ok(Self { arr, from: rng.start() })
	}
}

impl<'a> ZopfliChunk<'a> {
	/// # Full Slice.
	pub(crate) const fn arr(&self) -> &[u8] { self.arr }

	/// # Block Slice.
	pub(crate) fn block(&self) -> &[u8] {
		#[allow(unsafe_code)]
		// Safety: from is verified during construction.
		unsafe { self.arr.get_unchecked(self.from..) }
	}

	/// # First Value.
	///
	/// As blocks are non-empty, this always yields something.
	pub(crate) const fn first(&self) -> u8 {
		// Safety: from is verified during construction.
		if self.from >= self.arr.len() { crate::unreachable(); }
		self.arr[self.from]
	}

	/// # Active Length.
	pub(crate) const fn block_size(&self) -> NonZeroUsize {
		#[allow(unsafe_code)]
		// Safety: the length is verified during construction.
		unsafe { NonZeroUsize::new_unchecked(self.arr.len() - self.from) }
	}

	/// # Current Position.
	pub(crate) const fn pos(&self) -> usize { self.from }

	/// # Total Length.
	pub(crate) const fn total_len(&self) -> NonZeroUsize {
		#[allow(unsafe_code)]
		// Safety: slices are verified non-empty at construction.
		unsafe { NonZeroUsize::new_unchecked(self.arr.len()) }
	}

	#[allow(unsafe_code)]
	/// # Warmup Values.
	///
	/// This returns the first one or two values from `window_start`, used for
	/// warming up the hash.
	pub(crate) const fn warmup_values(&self) -> (u8, Option<u8>) {
		// Safety: from (and by association window_start) is verified at
		// construction.
		let window_start = self.window_start();
		if window_start >= self.arr.len() { crate::unreachable(); }

		let a = self.arr[window_start];

		// There will usually be a second value, but not always!
		let b =
			if window_start + 1 < self.arr.len() { Some(self.arr[window_start + 1]) }
			else { None };

		(a, b)
	}

	/// # Window Start.
	pub(crate) const fn window_start(&self) -> usize {
		self.from.saturating_sub(ZOPFLI_WINDOW_SIZE)
	}
}

impl<'a> ZopfliChunk<'a> {
	/// # Reducing Prelude Iterator.
	///
	/// Same as `ZopfliChunk::reducing_block_iter`, except for the prelude
	/// (`window_start..from`). Returns `None` if there is no prelude.
	pub(crate) fn reducing_prelude_iter(self) -> Option<std::iter::Take<ZopfliChunkIter<'a>>> {
		// If we're at the start of the slice, there is no prelude.
		if self.from == 0 { None }
		else {
			// Safety: from (and by association window_start) is verified at
			// construction.
			let window_start = self.window_start();
			if window_start >= self.arr.len() { crate::unreachable(); }

			let arr =
				if self.arr.len() - window_start <= ZOPFLI_MASTER_BLOCK_SIZE { self.arr }
				else { &self.arr[..window_start + ZOPFLI_MASTER_BLOCK_SIZE] };

			let chunk = Self { arr, from: window_start };
			Some(ZopfliChunkIter(chunk).take(self.from - window_start))
		}
	}

	/// # Reducing Block Chunk Iterator.
	///
	/// Return an iterator increasing the block start position after each
	/// cycle. (The first result is the current chunk.)
	pub(crate) const fn reducing_block_iter(self) -> ZopfliChunkIter<'a> {
		ZopfliChunkIter(self)
	}
}



/// # Chunk Iterator.
///
/// This iterator yields the seed chunk, then advances it and yields that,
/// etc., until the area of focus is empty.
pub(crate) struct ZopfliChunkIter<'a>(ZopfliChunk<'a>);

impl<'a> Iterator for ZopfliChunkIter<'a> {
	type Item = ZopfliChunk<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		// We potentially break the constraints during iteration so need to
		// explicitly check before returning.
		if self.0.from < self.0.arr.len() {
			let next = Some(self.0);
			self.0.from += 1;
			return next;
		}

		None
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.0.arr.len() - self.0.from;
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for ZopfliChunkIter<'a> {
	fn len(&self) -> usize { self.0.arr.len() - self.0.from }
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_reducing_block_iter() {
		let arr: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let chunk = ZopfliChunk { arr, from: 1 };
		let mut iter = chunk.reducing_block_iter();

		let mut len = 9;
		let mut from = 1;
		loop {
			// Check the iterator's presumed length.
			assert_eq!(iter.len(), len);
			if len == 0 { break; }

			// Pull the next entry and check the result.
			let next = iter.next().expect("reducing block iter terminated early");
			assert_eq!(next.block(), &arr[from..]);
			assert_eq!(next.pos(), from);

			len -= 1;
			from += 1;
		}

		// It should be empty.
		assert!(iter.next().is_none());
	}

	#[test]
	fn t_reducing_prelude_iter() {
		let arr: &[u8] = &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
		let chunk = ZopfliChunk { arr, from: 1 };
		let mut iter = chunk.reducing_prelude_iter().expect("missing prelude iter");

		assert_eq!(iter.len(), 1);
		let next = iter.next().expect("reducing prelude iter terminated early");
		assert_eq!(next.block(), arr);
		assert_eq!(next.pos(), 0);

		assert_eq!(iter.len(), 0);
		assert!(iter.next().is_none());

		// Let's try it again with a chunk that has no prelude.
		let chunk = ZopfliChunk { arr, from: 0 };
		assert!(chunk.reducing_prelude_iter().is_none());

		// And let's try one that is too big.
		let arr: &[u8] = &[0; ZOPFLI_MASTER_BLOCK_SIZE + 10];
		let chunk = ZopfliChunk::new(arr, 10).expect("Chunk failed.");
		let mut iter = chunk.reducing_prelude_iter().expect("missing prelude iter");

		assert_eq!(iter.len(), 10);
		let next = iter.next().expect("reducing prelude iter terminated early");

		// The slice should be truncated to fit the constraint.
		assert_eq!(next.block_size().get(), ZOPFLI_MASTER_BLOCK_SIZE);
	}
}
