/*!
# Flapfli: Huffman RLE Optimization.
*/

use dactyl::NoHash;
use std::{
	cell::{
		Cell,
		RefCell,
	},
	collections::{
		hash_map::Entry,
		HashMap,
	},
	num::NonZeroU32,
};
use super::{
	ArrayD,
	ArrayLL,
	best_tree_size,
	DeflateSym,
	DISTANCE_BITS,
	LengthLimitedCodeLengths,
	LZ77Store,
	ZopfliError,
	ZopfliRange,
};



/// # Length Symbol Extra Bits.
const LENGTH_EXTRA_BITS: [u32; 29] = [
	0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
	3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

type RleCache = HashMap<u64, CacheEntry, NoHash>;

thread_local!(
	/// # Best Tree Cache.
	///
	/// The dynamic length calculations are pretty terrible and can wind up
	/// being repeated several times for a given block. To take out some of the
	/// sting from that repetition, the results are statically cached.
	///
	/// To prevent endless reallocation and minimize lookup times, the cache is
	/// cleared for each new image.
	static CACHE: RefCell<RleCache> = RefCell::new(HashMap::default())
);



/// # Get Dynamic Lengths.
///
/// This method calculates the dynamic tree symbols and size using both the
/// existing and optimized counts, then returns whichever set produces the
/// smallest output.
///
/// Note: the returned size does not include the 3-bit block header.
pub(super) fn get_dynamic_lengths(store: &LZ77Store, rng: ZopfliRange)
-> Result<(u8, NonZeroU32, ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
	fn fetch(
		cache: &mut RleCache,
		ll_counts: &ArrayLL<u32>,
		d_counts: &ArrayD<u32>,
	) -> Result<(u8, NonZeroU32, ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
		// Pull the symbols, then get the sizes.
		let ll_lengths = ll_counts.llcl()?;
		let d_lengths = d_llcl(d_counts)?;
		let (data1, hash1) = calculate_size(cache, ll_counts, d_counts, &ll_lengths, &d_lengths)?;

		// Unless we've been here before and found optimization useless, repeat
		// the process using optimized counts and symbols.
		if ! data1.noop() {
			let (ll_lengths2, d_lengths2) = optimized_symbols(ll_counts, d_counts)?;
			let (data2, _) = calculate_size(cache, ll_counts, d_counts, &ll_lengths2, &d_lengths2)?;

			// Return this version if better.
			if data2.size < data1.size {
				return Ok((data2.extra(), data2.size, ll_lengths2, d_lengths2));
			}

			// Update the original's cache to reflect that optimization didn't
			// help so that we can skip all this the next time around.
			if let Some(e) = cache.get_mut(&hash1) { e.set_noop(); }
		}

		// The first version was better!
		Ok((data1.extra(), data1.size, ll_lengths, d_lengths))
	}

	// Pull the counts from the store.
	let (mut ll_counts, d_counts) = store.histogram(rng);
	ll_counts[256] = 1;

	// Do all the work!
	CACHE.with_borrow_mut(|cache| fetch(cache, &ll_counts, &d_counts))
}

/// # Reset Dynamic Length Cache.
///
/// To prevent endless reallocation and minimize lookup times, the cache is
/// cleared each time a new image is loaded.
pub(crate) fn reset_dynamic_length_cache() { CACHE.with_borrow_mut(HashMap::clear); }



#[derive(Clone, Copy)]
/// # Cache Entry.
struct CacheEntry {
	extra: u8,        // Extended alphabet used.
	size: NonZeroU32, // Combined tree/data size.
}

impl CacheEntry {
	/// # Extra Bits.
	///
	/// The first three bits comprise the extended alphabet details.
	const MASK_EXTRA: u8 = 0b0000_0111;

	/// # Fruitless Optimization Mask.
	///
	/// The fourth bit is used to indicate when the secondary optimization pass
	/// failed to result in better output.
	const MASK_NOOP: u8 = 0b0000_1000;

	/// # Extra.
	///
	/// Return the true "extra" value, without the noop bit.
	const fn extra(self) -> u8 { self.extra & Self::MASK_EXTRA }

	/// # Fruitless Optimization?
	///
	/// Returns true if optimizing the counts made no positive difference
	/// during the previous pass.
	const fn noop(self) -> bool { Self::MASK_NOOP == self.extra & Self::MASK_NOOP }

	/// # Set Fruitless Optimization.
	///
	/// This sets the noop flag so the optimization pass can be skipped on
	/// subsequent calls.
	fn set_noop(&mut self) { self.extra |= Self::MASK_NOOP; }
}



/// # RLE-Optimized Stretches.
///
/// This iterator yields a boolean value for each entry of the source slice,
/// `true` for distance codes in a sequence of 5+ zeroes or 7+ (identical)
/// non-zeroes, `false` otherwise.
///
/// This moots the need to collect such values into a vector in advance and
/// reduces the number of passes required to optimize Huffman codes.
struct GoodForRle<'a> {
	counts: &'a [Cell<u32>],
	good: usize,
	bad: usize,
}

impl<'a> GoodForRle<'a> {
	/// # New Instance.
	const fn new(counts: &'a [Cell<u32>]) -> Self {
		Self { counts, good: 0, bad: 0 }
	}
}

impl<'a> Iterator for GoodForRle<'a> {
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

impl<'a> ExactSizeIterator for GoodForRle<'a> {
	fn len(&self) -> usize { self.good + self.bad + self.counts.len() }
}



/// # Calculate Size.
///
/// Pull the best tree details from the cache, or calculate them fresh (and
/// cache them for next time).
fn calculate_size(
	cache: &mut RleCache,
	ll_counts: &ArrayLL<u32>,
	d_counts: &ArrayD<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> Result<(CacheEntry, u64), ZopfliError> {
	// Hash the symbols.
	let hash = deflate_hash(ll_counts, d_counts, ll_lengths, d_lengths);

	// Check the cache first.
	let entry = match cache.entry(hash) {
		Entry::Occupied(e) => return Ok((*e.get(), hash)),
		Entry::Vacant(e) => e,
	};

	// Calculate the sizes.
	let (extra, treesize) = best_tree_size(ll_lengths, d_lengths)?;
	let datasize = calculate_size_data(ll_counts, d_counts, ll_lengths, d_lengths);
	let size = treesize.saturating_add(datasize);
	let out = CacheEntry { extra, size };

	// Save to cache for later, then return.
	entry.insert(out);
	Ok((out, hash))
}

#[inline(never)]
/// # Calculate Dynamic Data Block Size.
fn calculate_size_data(
	ll_counts: &ArrayLL<u32>,
	d_counts: &ArrayD<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> u32 {
	// The early lengths and counts.
	let a = ll_lengths.iter().copied()
		.zip(ll_counts.iter().copied())
		.take(256)
		.map(|(ll, lc)| (ll as u32) * lc)
		.sum::<u32>();

	// The lengths and counts with extra bits.
	let b = ll_lengths[257..].iter().copied()
		.zip(ll_counts[257..].iter().copied())
		.zip(LENGTH_EXTRA_BITS)
		.map(|((ll, lc), lbit)| (ll as u32 + lbit) * lc)
		.sum::<u32>();

	// The distance lengths, counts, and extra bits.
	let c = d_lengths.iter().copied()
		.zip(d_counts.iter().copied())
		.zip(DISTANCE_BITS)
		.take(30)
		.map(|((dl, dc), dbit)| (dl as u32 + u32::from(dbit)) * dc)
		.sum::<u32>();

	a + b + c + ll_lengths[256] as u32
}

/// # Dynamic Length-Limited Code Lengths.
///
/// Calculate, patch, and return the distance code length symbols.
fn d_llcl(d_counts: &ArrayD<u32>)
-> Result<ArrayD<DeflateSym>, ZopfliError> {
	let mut d_lengths = d_counts.llcl()?;

	// Buggy decoders require at least two non-zero distances. Let's see
	// what we've got!
	let mut one: Option<bool> = None;
	for (i, dist) in d_lengths.iter().copied().enumerate().take(30) {
		// We have (at least) two non-zero entries; no patching needed!
		if ! dist.is_zero() && one.replace(i == 0).is_some() { return Ok(d_lengths); }
	}

	match one {
		// The first entry had a code, so patching the second gives us two.
		Some(true) => { d_lengths[1] = DeflateSym::D01; },
		// The first entry didn't have a code, so patching it gives us two.
		Some(false) => { d_lengths[0] = DeflateSym::D01; },
		// There were no codes at all, so we can just patch the first two.
		None => {
			d_lengths[0] = DeflateSym::D01;
			d_lengths[1] = DeflateSym::D01;
		},
	}

	Ok(d_lengths)
}

/// # Hash Counts and Symbols.
///
/// Calculate and return a hash for the set. This is done independently of the
/// map to reduce its signature and enable us to quickly repeat lookups if
/// necessary.
///
/// Note: both passes from a given dynamic lengths call will have the same
/// counts, but they hash quickly enough there's no performance benefit from
/// over-complicated the formula.
fn deflate_hash(
	ll_counts: &ArrayLL<u32>,
	d_counts: &ArrayD<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> u64 {
	use ahash::RandomState;
	use std::hash::{BuildHasher, Hash, Hasher};

	#[allow(unsafe_code)]
	/// # As Bytes.
	///
	/// Convert a `DeflateSym` array into an equivalent byte array for faster
	/// hashing.
	const fn deflate_bytes<const N: usize>(arr: &[DeflateSym; N]) -> &[u8; N] {
		// Safety: DeflateSym has the same size and alignment as u8, and if
		// for some reason that isn't true, this code won't compile.
		const {
			assert!(std::mem::size_of::<[DeflateSym; N]>() == std::mem::size_of::<[u8; N]>());
			assert!(std::mem::align_of::<[DeflateSym; N]>() == std::mem::align_of::<[u8; N]>());
		}
		unsafe { &* arr.as_ptr().cast() }
	}

	let mut h = RandomState::with_seeds(
		0x8596_cc44_bef0_1aa0,
		0x98d4_0948_da60_19ae,
		0x49f1_3013_c503_a6aa,
		0xc4d7_82ff_3c9f_7bef,
	).build_hasher();

	ll_counts.hash(&mut h);
	d_counts.hash(&mut h);
	deflate_bytes(ll_lengths).hash(&mut h);
	deflate_bytes(d_lengths).hash(&mut h);

	h.finish()
}

/// # Get RLE-Optimized Symbols.
///
/// Copy and optimize the counts, then recrunch and return their length-limited
/// symbols (but not the counts as they serve no further purpose).
fn optimized_symbols(ll_counts: &ArrayLL<u32>, d_counts: &ArrayD<u32>)
-> Result<(ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
	#[inline(never)]
	fn optimized_counts<const N: usize>(counts: &[u32; N]) -> [u32; N] {
		let mut counts2 = *counts;
		optimize_huffman_for_rle(&mut counts2);
		counts2
	}

	let ll_counts2 = optimized_counts(ll_counts);
	let d_counts2 = optimized_counts(d_counts);
	let ll_lengths2 = ll_counts2.llcl()?;
	let d_lengths2 = d_llcl(&d_counts2)?;

	Ok((ll_lengths2, d_lengths2))
}

#[allow(clippy::inline_always, clippy::integer_division)]
#[inline(always)]
/// # Optimize Huffman RLE Compression.
///
/// Change the population counts to potentially improve Huffman tree
/// compression, particularly the RLE part.
fn optimize_huffman_for_rle(mut counts: &mut [u32]) {
	// Convert counts to a proper slice with trailing zeroes trimmed.
	while let [ rest @ .., 0 ] = counts { counts = rest; }
	if counts.is_empty() { return; }

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
				let v = u32::max((sum + stride / 2) / stride, 1);
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
				|c| c.iter().fold(2, |a, c| a + c.get()) / 4
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
			let good = good.collect::<Vec<bool>>();
			assert_eq!(
				good.len(),
				c.len(),
				"Collected GoodForRle iterator count does not match source.",
			);
		}
	}
}
