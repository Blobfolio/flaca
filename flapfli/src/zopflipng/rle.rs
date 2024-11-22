/*!
# Flapfli: Huffman RLE Optimization.
*/

use std::{
	cell::Cell,
	num::NonZeroU32,
};
use super::{
	ArrayD,
	ArrayLL,
	best_tree_size,
	DeflateSym,
	LengthLimitedCodeLengths,
	LZ77StoreRange,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
};



/// # Distance Extra Byts (by Symbol).
const DISTANCE_BITS: &ArrayD<u32> = &[
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
	7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

/// # Length Symbol Extra Bits.
const LENGTH_EXTRA_BITS: &ArrayLL<u32> = &[
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
	0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
	0, 0,
];



/// # Dynamic Lengths.
///
/// This struct is used to perform brute-force length-limited-code-length
/// calculations to determine the best (smallest) DEFLATE configuration for the
/// data.
///
/// This is done in two passes: the first using the previously-collected LZ77
/// histogram data, the second using RLE-optimized counts derived from same.
/// The best of the best is kept, the rest are forgotten.
pub(crate) struct DynamicLengths {
	/// # Extra Deflate Symbols Used.
	extra: u8,

	/// # Total Size.
	size: NonZeroU32,

	/// # Litlen Counts.
	ll_lengths: ArrayLL<DeflateSym>,

	/// # Distance Counts.
	d_lengths: ArrayD<DeflateSym>,
}

impl DynamicLengths {
	/// # New.
	pub(crate) fn new(store: LZ77StoreRange) -> Result<Self, ZopfliError> {
		// Pull the counts from the store.
		let (ll_counts, d_counts) = store.histogram();

		// Pull the symbols, then get the sizes.
		let ll_lengths = ll_counts.llcl()?;
		let d_lengths = d_counts.llcl()?;

		// Calculate the sizes.
		let (extra, size) = calculate_size(&ll_counts, &d_counts, &ll_lengths, &d_lengths)?;
		let mut out = Self { extra, size, ll_lengths, d_lengths };

		if let Some((ll_lengths2, d_lengths2)) = out.try_optimized(&ll_counts, &d_counts) {
			// Calculate the sizes.
			let (extra2, size2) = calculate_size(&ll_counts, &d_counts, &ll_lengths2, &d_lengths2)?;

			// Update our values if the new cost is lower.
			if size2 < out.size {
				out.extra = extra2;
				out.size = size2;
				out.ll_lengths = ll_lengths2;
				out.d_lengths = d_lengths2;
			}
		}

		// Done!
		Ok(out)
	}

	#[expect(clippy::option_if_let_else, reason = "Too messy.")]
	/// # Try Optimized.
	///
	/// Optimize the counts and fetch new symbols, returning them unless
	/// neither wind up any different.
	fn try_optimized(&self, ll_counts: &ArrayLL<u32>, d_counts: &ArrayD<u32>)
	-> Option<(ArrayLL<DeflateSym>, ArrayD<DeflateSym>)> {
		#[expect(unsafe_code, reason = "For pointer deref.")]
		/// # As Bytes.
		///
		/// Reimagine a symbol array as raw bytes for more optimal comparison.
		const fn deflate_bytes<const N: usize>(arr: &[DeflateSym; N]) -> &[u8; N] {
			// Safety: DeflateSym has the same size and alignment as u8.
			unsafe { &* arr.as_ptr().cast() }
		}

		// Let's start with the distances because they're cheaper to compare
		// and copy.
		let mut unique = false;
		let d_lengths2 = optimize_huffman_for_rle(d_counts)
			.map_or(self.d_lengths, |l| {
				unique = *deflate_bytes(&self.d_lengths) != *deflate_bytes(&l);
				l
			});

		// And now the lengths!
		if let Some(l) = optimize_huffman_for_rle(ll_counts) {
			if unique || deflate_bytes(&self.ll_lengths) != deflate_bytes(&l) {
				Some((l, d_lengths2))
			}
			else { None }
		}
		else if unique { Some((self.ll_lengths, d_lengths2)) }
		else { None }
	}
}

impl DynamicLengths {
	/// # Cost.
	pub(crate) const fn cost(&self) -> NonZeroU32 { self.size }

	/// # Extra.
	pub(crate) const fn extra(&self) -> u8 { self.extra }

	/// # Litlen Lengths.
	pub(crate) const fn ll_lengths(&self) -> &ArrayLL<DeflateSym> { &self.ll_lengths }

	/// # Distance Lengths.
	pub(crate) const fn d_lengths(&self) -> &ArrayD<DeflateSym> { &self.d_lengths }

	/// # Take Size.
	///
	/// Same as `DynamicLengths::cost`, but drop `self` in the process.
	pub(crate) const fn take_size(self) -> NonZeroU32 { self.size }
}



/// # Calculate Size and Extra.
fn calculate_size(
	ll_counts: &ArrayLL<u32>,
	d_counts: &ArrayD<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> Result<(u8, NonZeroU32), ZopfliError> {
	// Tree size.
	let (extra, treesize) = best_tree_size(ll_lengths, d_lengths)?;

	// Data size.
	debug_assert!(ll_counts[256] == 1, "BUG: symbol 256 is not one?!"); // .histogram() should set this.
	let a = DataSizeIter::new(ll_counts, ll_lengths, LENGTH_EXTRA_BITS).sum::<u32>();
	let b = DataSizeIter::new(d_counts, d_lengths, DISTANCE_BITS).sum::<u32>();

	// Total size.
	let size = treesize.saturating_add(a + b);

	Ok((extra, size))
}

#[expect(clippy::integer_division, reason = "We want this.")]
/// # Optimize Huffman RLE Compression.
///
/// Change the population counts to potentially improve Huffman tree
/// compression, particularly the RLE part.
fn optimize_huffman_for_rle<const N: usize>(counts: &[u32; N]) -> Option<[DeflateSym; N]>
where [u32; N]: LengthLimitedCodeLengths<N> {
	const {
		assert!(
			N == ZOPFLI_NUM_D || N == ZOPFLI_NUM_LL,
			"BUG: counts must have a length of 32 or 288.",
		);
	}

	let mut counts2 = *counts;
	let mut counts = counts2.as_mut_slice();

	// Convert counts to a proper slice with trailing zeroes trimmed.
	while let [ rest @ .., 0 ] = counts { counts = rest; }
	if counts.is_empty() { return None; }

	// We need to read and write simultaneously; once again the Cell trick can
	// keep us safe!
	let counts = Cell::from_mut(counts).as_slice_of_cells();

	// Find collapseable ranges!
	let mut stride: u32 = 0;
	let mut scratch: u32 = counts[0].get();
	let mut sum: u32 = 0;
	for (i, (count, good)) in counts.iter().map(Cell::get).zip(GoodForRle::new(counts)).enumerate() {
		// Time to reset (and maybe collapse).
		if good || count.abs_diff(scratch) >= 4 {
			// Collapse the stride if it is as least four and contained
			// something non-zero.
			if sum != 0 && stride >= 4 {
				let v = u32::max((sum + stride.wrapping_div(2)).wrapping_div(stride), 1);
				// This condition just helps the compiler understand the range
				// won't overflow; it can't, but it doesn't know that.
				if let Some(from) = i.checked_sub(stride as usize) {
					for c in &counts[from..i] { c.set(v); }
				}
			}

			// Reset!
			stride = 0;
			sum = 0;

			// If there are at least three future counts, we can set scratch
			// to a sorted weighted average, otherwise the current value will
			// do.
			scratch = counts.get(i..i + 4).map_or(
				count,
				|c| c.iter().fold(2, |a, c| a + c.get()).wrapping_div(4)
			);
		}

		stride += 1;
		sum += count;
	}

	// Collapse the trailing stride, if any.
	if sum != 0 && stride >= 4 {
		let v = u32::max((sum + stride / 2) / stride, 1);
		// This condition just helps the compiler understand the range won't
		// overflow; it can't, but it doesn't know that.
		if let Some(from) = counts.len().checked_sub(stride as usize) {
			for c in &counts[from..] { c.set(v); }
		}
	}

	// LLCL time!
	counts2.llcl().ok()
}



/// # Data Size Iterator.
///
/// This iterator yields the combined data size for all but the last two
/// length/count/bit triplets, because why would zopfli ever utilize all of the
/// data it collects?!
///
/// This is only used by `calculate_size_data`. Traditional iterators get a
/// little clunky with all the zipping and copying and mapping.
struct DataSizeIter<'a, const N: usize> {
	/// # Counts.
	counts:  &'a [u32; N],

	/// # Lengths.
	lengths: &'a [DeflateSym; N],

	/// # Bits.
	bits:    &'a [u32; N],

	/// # Current Index.
	pos: usize,
}

impl<'a, const N: usize> DataSizeIter<'a, N> {
	/// # New.
	const fn new(counts: &'a [u32; N], lengths: &'a [DeflateSym; N], bits: &'a [u32; N])
	-> Self {
		const { assert!(2 < N, "BUG: there must be at least two leaves."); }
		Self { counts, lengths, bits, pos: 0 }
	}
}

impl<const N: usize> Iterator for DataSizeIter<'_, N> {
	type Item = u32;

	#[inline]
	fn next(&mut self) -> Option<Self::Item> {
		let idx = self.pos;
		if idx + 2 < N {
			self.pos += 1;
			Some(self.counts[idx] * (self.lengths[idx] as u32 + self.bits[idx]))
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<const N: usize> ExactSizeIterator for DataSizeIter<'_, N> {
	fn len(&self) -> usize { N - 2 - self.pos }
}



/// # RLE-Optimized Stretches.
///
/// This iterator yields a boolean value for each entry of the source slice,
/// `true` for distance codes in a sequence of 5+ zeroes or 7+ (identical)
/// non-zeroes, `false` otherwise.
///
/// This moots the need to collect the values into a vector in advance and
/// reduces the number of passes required to optimize Huffman codes.
struct GoodForRle<'a> {
	/// # Counts.
	counts: &'a [Cell<u32>],

	/// # Good Buffer.
	///
	/// Leftover results from previous iterations, returned when non-zero.
	good: usize,

	/// # Bad Buffer.
	///
	/// Leftover results from previous iterations, returned when non-zero.
	bad: usize,
}

impl<'a> GoodForRle<'a> {
	/// # New Instance.
	const fn new(counts: &'a [Cell<u32>]) -> Self {
		Self { counts, good: 0, bad: 0 }
	}
}

impl Iterator for GoodForRle<'_> {
	type Item = bool;

	fn next(&mut self) -> Option<Self::Item> {
		// Return good or bad values from the buffer.
		if self.good != 0 {
			self.good -= 1;
			return Some(true);
		}
		if self.bad != 0 {
			self.bad -= 1;
			return Some(false);
		}

		// If the slice is empty, we're done!
		if self.counts.is_empty() { return None; }

		// See how many times the next entry is repeated, if at all, shortening
		// the slice accordingly.
		let scratch = self.counts[0].get();
		let mut stride = 0;
		while let [count, rest @ ..] = self.counts {
			// Note the reptition and circle back around. This will always
			// trigger on the first pass, so stride will always be at least
			// one.
			if count.get() == scratch {
				stride += 1;
				self.counts = rest;
			}
			// We had an optimal stretch.
			else if stride >= 5 && (scratch == 0 || stride >= 7) {
				self.good = stride - 1;
				return Some(true);
			}
			// We had a non-optimal stretch.
			else {
				self.bad = stride - 1;
				return Some(false);
			}
		}

		// Finish up by qualifying the dangling stride as optimal or not.
		if stride >= 5 && (scratch == 0 || stride >= 7) {
			self.good = stride - 1;
			Some(true)
		}
		else {
			self.bad = stride - 1;
			Some(false)
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl ExactSizeIterator for GoodForRle<'_> {
	fn len(&self) -> usize { self.good + self.bad + self.counts.len() }
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_good_for_rle() {
		for c in [
			[196, 23, 10, 12, 5, 4, 1, 23, 8, 2, 6, 5, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 47, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 8, 1, 1, 1, 0, 4, 7, 2, 0, 1, 10, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 42, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 44, 284, 1, 15, 21, 0, 55, 0, 19, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 320, 12, 0, 0, 17, 3, 0, 3, 2].as_mut_slice(),
			[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 0, 288, 11, 41, 6, 5, 2, 0, 0, 0, 1].as_mut_slice(),
			[201, 24, 10, 12, 5, 4, 1, 24, 8, 2, 6, 4, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 49, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 9, 1, 1, 1, 0, 4, 6, 2, 0, 1, 8, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 44, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 46, 281, 1, 20, 3, 10, 59, 0, 4, 12, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 318, 12, 0, 0, 21, 0, 0, 3, 2].as_mut_slice(),
		] {
			let c = Cell::from_mut(c).as_slice_of_cells();

			// Make sure our ExactSizeness is working.
			let good = GoodForRle::new(c);
			assert_eq!(
				good.len(),
				c.len(),
				"GoodForRle iterator count does not match source.",
			);

			// And make sure we actually collect that count!
			assert_eq!(
				good.count(),
				c.len(),
				"Collected GoodForRle iterator count does not match source.",
			);
		}
	}
}
