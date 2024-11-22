/*!
# Flapfli: Miscellaneous Iterators.
*/



/// # Reducing Slice Iterator.
///
/// This iterator yields all non-empty slices spanning `n..`, incrementing `n`
/// by one after each cycle.
///
/// In other words, this will start by returning the original slice, then `1..`,
/// `2..`, `3..`, etc., stopping when empty.
pub(super) struct ReducingSlices<'a, T>(&'a [T]);

impl<'a, T> ReducingSlices<'a, T> {
	/// # New.
	pub(super) const fn new(arr: &'a [T]) -> Self { Self(arr) }
}

impl<'a, T> Iterator for ReducingSlices<'a, T> {
	type Item = &'a [T];

	fn next(&mut self) -> Option<Self::Item> {
		if let [_, rest @ ..] = &self.0 {
			Some(std::mem::replace(&mut self.0, rest))
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.0.len();
		(len, Some(len))
	}
}

impl<T> ExactSizeIterator for ReducingSlices<'_, T> {
	#[inline]
	fn len(&self) -> usize { self.0.len() }
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_reducing_slices() {
		let slice: &[u8] = &[0, 1, 2, 3, 4, 5];
		let mut reducing = ReducingSlices::new(slice);

		assert_eq!(reducing.len(), slice.len());
		assert_eq!(reducing.next(), Some(slice));

		assert_eq!(reducing.len(), 5);
		assert_eq!(reducing.next(), Some(&slice[1..]));

		assert_eq!(reducing.len(), 4);
		assert_eq!(reducing.next(), Some(&slice[2..]));

		assert_eq!(reducing.len(), 3);
		assert_eq!(reducing.next(), Some(&slice[3..]));

		assert_eq!(reducing.len(), 2);
		assert_eq!(reducing.next(), Some(&slice[4..]));

		assert_eq!(reducing.len(), 1);
		assert_eq!(reducing.next(), Some(&slice[5..]));

		assert_eq!(reducing.len(), 0);
		assert_eq!(reducing.next(), None);
	}
}
