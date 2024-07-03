/*!
# Flapfli: Ranges.
*/

use std::{
	num::{
		NonZeroU32,
		NonZeroUsize,
	},
	ops::Range,
};
use super::{
	zopfli_error,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliError,
};



#[derive(Debug, Clone, Copy)]
/// # Block Range.
///
/// This struct exists primarily to guarantee a range is non-empty and no
/// larger than `ZOPFLI_MASTER_BLOCK_SIZE`.
///
/// It also implements `Copy`, so there's that. Haha.
pub(crate) struct ZopfliRange {
	start: usize,
	end: usize,
}

impl ZopfliRange {
	/// # New.
	pub(crate) const fn new(start: usize, end: usize) -> Result<Self, ZopfliError> {
		if start < end && end - start <= ZOPFLI_MASTER_BLOCK_SIZE {
			Ok(Self { start, end })
		}
		else { Err(zopfli_error!()) }
	}

	/// # Update.
	///
	/// Adjust the start and end positions if they uphold the constraints,
	/// otherwise return an error.
	pub(crate) fn set(&mut self, start: usize, end: usize) -> Result<(), ZopfliError> {
		if start < end && end - start <= ZOPFLI_MASTER_BLOCK_SIZE {
			self.start = start;
			self.end = end;
			Ok(())
		}
		else { Err(zopfli_error!()) }
	}
}

impl ZopfliRange {
	/// # Start.
	pub(crate) const fn start(&self) -> usize { self.start }

	/// # End.
	pub(crate) const fn end(&self) -> usize { self.end }

	/// # As Range.
	pub(crate) const fn rng(&self) -> Range<usize> { self.start..self.end }

	#[allow(unsafe_code)]
	/// # Length.
	pub(crate) const fn len(&self) -> NonZeroUsize {
		// Safety: we verified start is less than end during construction.
		unsafe { NonZeroUsize::new_unchecked(self.end - self.start) }
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Length.
	pub(crate) const fn len32(&self) -> NonZeroU32 {
		// Safety: we verified start is less than end during construction, and
		// the total is within a million.
		unsafe { NonZeroU32::new_unchecked((self.end - self.start) as u32) }
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_range() {
		// Some simple bad ranges.
		assert!(ZopfliRange::new(0, 0).is_err());
		assert!(ZopfliRange::new(3, 2).is_err());
		assert!(ZopfliRange::new(0, ZOPFLI_MASTER_BLOCK_SIZE + 1).is_err());

		// This should fit!.
		assert!(ZopfliRange::new(0, ZOPFLI_MASTER_BLOCK_SIZE).is_ok());

		// Let's test the getters.
		let rng = ZopfliRange::new(1, 5).expect("Range failed!");
		assert_eq!(rng.start(), 1);
		assert_eq!(rng.end(), 5);
		assert_eq!(rng.len(), NonZeroUsize::new(4).unwrap());
		assert_eq!(rng.rng(), 1..5);
	}
}
