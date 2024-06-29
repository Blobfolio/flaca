/*!
# Flapfli: Ranges.
*/

use std::{
	num::NonZeroUsize,
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

	/// # Advance.
	///
	/// Return a new range starting one spot later, unless that violates the
	/// constraints.
	pub(crate) const fn advance(&self) -> Result<Self, ZopfliError> {
		if 1 < self.end - self.start { Ok(Self { start: self.start + 1, end: self.end }) }
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

	/// # Split.
	///
	/// Split the range into `start..mid` and `mid..end`, unless `mid` is equal
	/// to one or the other extreme.
	pub(crate) const fn split(&self, mid: usize) -> Result<(Self, Self), ZopfliError> {
		if self.start < mid && mid < self.end {
			Ok((
				Self { start: self.start, end: mid },
				Self { start: mid, end: self.end }
			))
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

	/// # Splits Iterator.
	pub(crate) const fn splits(&self) -> ZopfliRangeSplits {
		ZopfliRangeSplits {
			start: self.start,
			splits: self.start + 1..self.end,
		}
	}
}



/// # Split Ranges.
///
/// This iterator yields all possible non-empty splits for a `ZopfliRange`; it
/// is used to short-circuit minimum cost calculations.
pub(crate) struct ZopfliRangeSplits {
	start: usize,
	splits: Range<usize>,
}

impl Iterator for ZopfliRangeSplits {
	type Item = (ZopfliRange, ZopfliRange);

	fn next(&mut self) -> Option<Self::Item> {
		let mid = self.splits.next()?;
		Some((
			ZopfliRange { start: self.start, end: mid },
			ZopfliRange { start: mid, end: self.splits.end },
		))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.splits.len();
		(len, Some(len))
	}
}

impl ExactSizeIterator for ZopfliRangeSplits {
	fn len(&self) -> usize { self.splits.len() }
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

		// Let's make sure splitting works as expected too.
		let mut splits = rng.splits();
		assert_eq!(splits.len(), 3);
		assert_eq!(
			splits.next().map(|(a, b)| (a.rng(), b.rng())),
			Some((1..2, 2..5)),
		);
		assert_eq!(
			splits.next().map(|(a, b)| (a.rng(), b.rng())),
			Some((1..3, 3..5)),
		);
		assert_eq!(
			splits.next().map(|(a, b)| (a.rng(), b.rng())),
			Some((1..4, 4..5)),
		);
		assert!(splits.next().is_none());
	}
}
