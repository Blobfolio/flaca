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
/// included for the ride because it is sometimes relevant for hashing and
/// caching.
///
/// Similar to `ZopfliRange`, this struct mainly exists to help enforce the
/// logical constraints so we don't have to repeat sanity checks every five
/// seconds.
///
/// The struct's `from` value may or may not be zero — on the first pass there
/// won't be any previous data — but it will always be less than `arr.len()`,
/// and `arr.len() - from` will always be less than or equal to
/// `ZOPFLI_MASTER_BLOCK_SIZE`, i.e. one million.
pub(crate) struct ZopfliChunk<'a> {
	/// # Array.
	arr: &'a [u8],

	/// # Window Start.
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
	/// out of bounds or otherwise violates the struct's requirements.
	///
	/// The `start` serves as the new instances `from`. If `end` is less than
	/// `arr.len()`, the new chunk's slice will be truncated accordingly.
	pub(crate) fn reslice(&self, start: usize, end: usize) -> Result<Self, ZopfliError> {
		if start < end && end - start <= ZOPFLI_MASTER_BLOCK_SIZE && end <= self.arr.len() {
			let arr = &self.arr[..end];
			Ok(Self { arr, from: start })
		}
		else { Err(zopfli_error!()) }
	}

	/// # Reslice to Range.
	///
	/// Same as `ZopfliChunk::reslice`, but with the range specified as a
	/// `ZopfliRange`.
	///
	/// This version should be preferred in cases where such a range has
	/// already been constructed since it moots all but one of the conditions
	/// we'd otherwise need to verify before giving the `Ok()`.
	pub(crate) fn reslice_rng(&self, rng: ZopfliRange) -> Result<Self, ZopfliError> {
		let arr = self.arr.get(..rng.end()).ok_or(zopfli_error!())?;
		Ok(Self { arr, from: rng.start() })
	}
}

impl ZopfliChunk<'_> {
	/// # Full Slice.
	///
	/// Return the entire data slice, including the prelude, if any.
	///
	/// Note: this will never be empty.
	pub(crate) const fn arr(&self) -> &[u8] { self.arr }

	/// # Block Slice.
	///
	/// Return the "active" portion of the data slice, i.e. everything from
	/// `from`.
	///
	/// Note: this will never be empty.
	pub(crate) fn block(&self) -> &[u8] {
		#[expect(unsafe_code, reason = "For performance.")]
		// Safety: from is verified during construction.
		unsafe { self.arr.get_unchecked(self.from..) }
	}

	/// # First Value.
	///
	/// Return the first value from the "active" portion of the data slice,
	/// i.e. `arr[from]`.
	///
	/// Because the current block may never be empty, there will always be at
	/// least one value.
	pub(crate) const fn first(&self) -> u8 {
		if self.arr.len() < self.from { 0 } // Impossible.
		else { self.arr[self.from] }
	}

	/// # Active Length.
	///
	/// Return the length of the "active" slice, e.g. its block size.
	pub(crate) const fn block_size(&self) -> NonZeroUsize {
		#[expect(unsafe_code, reason = "For performance.")]
		// Safety: the length is verified during construction.
		unsafe { NonZeroUsize::new_unchecked(self.arr.len() - self.from) }
	}

	/// # Current Position.
	///
	/// Return the `from` index that marks the starting point of the "active"
	/// portion of the data slice.
	pub(crate) const fn pos(&self) -> usize { self.from }

	/// # Total Length.
	///
	/// Return the length of the entire data slice, prelude and all.
	pub(crate) const fn total_len(&self) -> NonZeroUsize {
		#[expect(unsafe_code, reason = "For performance.")]
		// Safety: slices are verified non-empty at construction.
		unsafe { NonZeroUsize::new_unchecked(self.arr.len()) }
	}

	/// # Warmup Values.
	///
	/// This returns the first one or two values from `window_start`, used for
	/// warming up the `ZopfliHash` cache.
	///
	/// Note: it is probably impossible for there to not be a second value, but
	/// since we don't explicitly require lengths of two, it's safer to treat
	/// it as optional.
	pub(crate) const fn warmup_values(&self) -> (u8, Option<u8>) {
		let window_start = self.window_start();
		if window_start >= self.arr.len() { return (0, None); } // Impossible.

		let a = self.arr[window_start];

		// There will usually be a second value, but not always!
		let b =
			if window_start + 1 < self.arr.len() { Some(self.arr[window_start + 1]) }
			else { None };

		(a, b)
	}

	/// # Window Start.
	///
	/// If we're at the beginning of a chunk, this is equivalent to
	/// `ZopfliChunk::pos` (e.g. `self.from`), otherwise it reaches back up to
	/// `ZOPFLI_WINDOW_SIZE` slots into the prelude, returning that index
	/// instead.
	pub(crate) const fn window_start(&self) -> usize {
		self.from.saturating_sub(ZOPFLI_WINDOW_SIZE)
	}
}

impl<'a> ZopfliChunk<'a> {
	/// # Reducing Prelude Iterator.
	///
	/// Same as `ZopfliChunk::reducing_block_iter`, except the chunks are
	/// restricted to the range of the prelude — `window_start..from` — if any.
	///
	/// If there is no prelude, `None` is returned instead.
	///
	/// Note: the internal slice will be truncated if needed to uphold the
	/// maximum length constraint, but that loss doesn't actually matter since
	/// prelude hashing never looks at more than `u16::MAX` bytes anyway. (A
	/// million minus thirty-odd thousand is still much more than that!)
	pub(crate) fn reducing_prelude_iter(self) -> Option<std::iter::Take<ZopfliChunkIter<'a>>> {
		// If we're at the start of the slice, there is no prelude.
		if self.from == 0 { None }
		else {
			let window_start = self.window_start();
			if window_start >= self.arr.len() { return None; } // Impossible.

			let arr =
				if self.arr.len() - window_start <= ZOPFLI_MASTER_BLOCK_SIZE { self.arr }
				else { &self.arr[..window_start + ZOPFLI_MASTER_BLOCK_SIZE] };

			let chunk = Self { arr, from: window_start };
			Some(ZopfliChunkIter(chunk).take(self.from - window_start))
		}
	}

	/// # Reducing Block Chunk Iterator.
	///
	/// Return an iterator that increases the block's starting position (`from`)
	/// after each pass, stopping once the chunk would be empty/invalid.
	///
	/// Similar to the more generic `ReducingSlice` iterator, this starts with
	/// the current value, so there will always be at least one valid result
	/// before `None`.
	pub(crate) const fn reducing_block_iter(self) -> ZopfliChunkIter<'a> {
		ZopfliChunkIter(self)
	}
}



/// # Chunk Iterator.
///
/// This iterator yields increasingly smaller chunks until empty, incrementing
/// the starting position by one after each cycle, beginning with the seed
/// chunk.
pub(crate) struct ZopfliChunkIter<'a>(ZopfliChunk<'a>);

impl<'a> Iterator for ZopfliChunkIter<'a> {
	type Item = ZopfliChunk<'a>;

	fn next(&mut self) -> Option<Self::Item> {
		// We potentially break the constraints during iteration so need to
		// explicitly check from is still in range and non-empty before
		// returning.
		if self.0.from < self.0.arr.len() {
			let next = Some(self.0);
			self.0.from += 1;
			next
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.0.arr.len() - self.0.from;
		(len, Some(len))
	}
}

impl ExactSizeIterator for ZopfliChunkIter<'_> {
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
		let arr = vec![0_u8; ZOPFLI_MASTER_BLOCK_SIZE + 10];
		let chunk = ZopfliChunk::new(arr.as_slice(), 10).expect("Chunk failed.");
		let mut iter = chunk.reducing_prelude_iter().expect("missing prelude iter");

		assert_eq!(iter.len(), 10);
		let next = iter.next().expect("reducing prelude iter terminated early");

		// The slice should be truncated to fit the constraint.
		assert_eq!(next.block_size().get(), ZOPFLI_MASTER_BLOCK_SIZE);
	}
}
