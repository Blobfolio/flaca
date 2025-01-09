/*!
# Flapfli: LZ77 Store.

This module defines the LZ77 store structures.
*/

use std::{
	num::{
		NonZeroU32,
		NonZeroUsize,
	},
	ops::Range,
};
use super::{
	ArrayD,
	ArrayLL,
	DISTANCE_BITS,
	DISTANCE_SYMBOLS,
	Dsym,
	DynamicLengths,
	FIXED_TREE_LL,
	LENGTH_SYMBOL_BITS,
	LENGTH_SYMBOLS,
	LitLen,
	Lsym,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	zopfli_error,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliError,
	ZopfliRange,
};



/// # Seven is Non-Zero.
const NZ07: NonZeroU32 = NonZeroU32::new(7).unwrap();

/// # Eight is Non-Zero.
const NZ08: NonZeroU32 = NonZeroU32::new(8).unwrap();



#[derive(Clone)]
/// # LZ77 Data Store.
///
/// This struct holds litlen, dist, and symbol information for LZ77 block
/// compression.
///
/// This can be thought of as the owned version of `LZ77StoreRange`, useful
/// while the data is still being gathered and manipulated.
pub(crate) struct LZ77Store {
	/// # Entries.
	pub(crate) entries: Vec<LZ77StoreEntry>,
}

impl LZ77Store {
	/// # Small Store Limit.
	pub(crate) const SMALL_STORE: usize = 1000;

	/// # New.
	pub(crate) const fn new() -> Self {
		Self { entries: Vec::new() }
	}

	/// # Ranged.
	///
	/// Return an immutable ranged view of the data, or an error if the range
	/// is invalid.
	pub(crate) fn ranged(&self, rng: ZopfliRange) -> Result<LZ77StoreRange, ZopfliError> {
		let entries = self.entries.get(rng.rng()).ok_or(zopfli_error!())?;
		Ok(LZ77StoreRange { entries })
	}

	/// # Ranged (Full).
	///
	/// Same as `LZ77Store::range`, except the range is everything. This will
	/// return an error if the store is empty or too large.
	pub(crate) fn ranged_full(&self) -> Result<LZ77StoreRange, ZopfliError> {
		let entries = self.entries.as_slice();
		if entries.is_empty() || ZOPFLI_MASTER_BLOCK_SIZE < entries.len() {
			Err(zopfli_error!())
		}
		else { Ok(LZ77StoreRange { entries }) }
	}

	/// # Clear.
	///
	/// Remove all previously-collected entries, allowing the store to be
	/// re-used for a new set of data.
	pub(crate) fn clear(&mut self) { self.entries.truncate(0); }

	/// # Push Values.
	///
	/// Create an entry from the arguments, then insert it into the store.
	pub(crate) fn push(&mut self, litlen: LitLen, dist: u16, pos: usize) {
		self.push_entry(LZ77StoreEntry::new(litlen, dist, pos));
	}

	/// # Push Entry.
	///
	/// Push an existing entry directly to the store.
	fn push_entry(&mut self, entry: LZ77StoreEntry) { self.entries.push(entry); }

	/// # Replace Store.
	///
	/// Replace the current store's data with what the other guy's got.
	pub(crate) fn replace(&mut self, other: &Self) {
		self.entries.clone_from(&other.entries);
	}

	/// # Steal/Append Entries.
	///
	/// Drain the entires from `other` and append them to `self`. (This is a
	/// more efficient alternative to calling `LZ77Store::replace` and
	/// `LZ77Store::clear` separately.)
	pub(crate) fn steal_entries(&mut self, other: &mut Self) {
		self.entries.append(&mut other.entries);
	}
}

impl LZ77Store {
	/// # Is Small?
	///
	/// Returns true if there are a thousand or fewer entries.
	pub(crate) fn is_small(&self) -> bool {
		self.entries.len() <= Self::SMALL_STORE
	}

	/// # Length.
	///
	/// Return the number of entries in the store. Unlike `LZ77StoreRange`,
	/// this can return zero.
	pub(crate) fn len(&self) -> usize { self.entries.len() }
}



#[derive(Clone, Copy)]
/// # Ranged LZ77 Data Store.
///
/// Same as `LZ77Store`, but immutable and non-empty, offering a more
/// const-friendly and performant view into some or all of the former's
/// data.
pub(crate) struct LZ77StoreRange<'a> {
	/// # Entries.
	pub(crate) entries: &'a [LZ77StoreEntry],
}

impl<'a> LZ77StoreRange<'a> {
	/// # Uncompressed Range.
	///
	/// Return the original uncompressed range — from e.g. a `ZopfliChunk` —
	/// used to build this store. If for some reason that range cannot be
	/// recreated, an error will be returned instead.
	pub(crate) const fn byte_range(self) -> Result<ZopfliRange, ZopfliError> {
		let len = self.entries.len();
		if 0 == len { return Err(zopfli_error!()); } // Ranged stores are never empty.

		let first = self.entries[0];
		let last = self.entries[len - 1];
		ZopfliRange::new(first.pos, last.length() as usize + last.pos)
	}

	/// # Histogram.
	///
	/// Count up and return the litlen and distance symbols included in this
	/// range.
	pub(crate) fn histogram(self) -> (ArrayLL<u32>, ArrayD<u32>) {
		let mut ll_counts = ZEROED_COUNTS_LL;
		let mut d_counts = ZEROED_COUNTS_D;

		for e in self.entries {
			ll_counts[e.ll_symbol as usize] += 1;
			if 0 < e.dist { d_counts[e.d_symbol as usize] += 1; }
		}

		// This should always be one.
		ll_counts[256] = 1;

		(ll_counts, d_counts)
	}

	/// # Is Small?
	///
	/// Returns true if there are a thousand or fewer entries.
	pub(crate) const fn is_small(&self) -> bool {
		self.entries.len() <= LZ77Store::SMALL_STORE
	}

	/// # Length.
	///
	/// Return the total number of entries included in this store. Unlike
	/// `LZ77Store`, this cannot be empty, so the result will always be
	/// non-zero.
	pub(crate) const fn len(self) -> NonZeroUsize {
		#[expect(unsafe_code, reason = "Entries are non-empty.")]
		// Safety: we verified the store is non-empty at construction.
		unsafe { NonZeroUsize::new_unchecked(self.entries.len()) }
	}

	/// # (re)Ranged.
	///
	/// Same as `LZ77Store::ranged`, but for stores that are already ranged.
	pub(crate) fn ranged(&self, rng: ZopfliRange) -> Result<Self, ZopfliError> {
		let entries = self.entries.get(rng.rng()).ok_or(zopfli_error!())?;
		Ok(Self { entries })
	}

	/// # Split Chunk Iterator.
	///
	/// Return an iterator that yields nine evenly-divided, non-empty split
	/// combinations for minimum-cost-testing, unless the store is too small
	/// to split.
	pub(crate) const fn splits_chunked(self) -> Option<LZ77StoreRangeSplitsChunked<'a>> {
		LZ77StoreRangeSplitsChunked::new(self.entries)
	}

	/// # Split Iterator.
	///
	/// Return an iterator that yields every possible split combination in
	/// order, unless `self` has only one entry and cannot be split, in which
	/// case an error is returned instead.
	pub(crate) const fn splits(self) -> Result<LZ77StoreRangeSplits<'a>, ZopfliError> {
		let len = self.entries.len();
		if 1 < len {
			Ok(LZ77StoreRangeSplits {
				entries: self.entries,
				splits: 1..len,
			})
		}
		// Not big enough to split!
		else { Err(zopfli_error!()) }
	}
}

impl LZ77StoreRange<'_> {
	/// # Calculate Block Size (Auto).
	///
	/// Return the smallest of the uncompressed, fixed, and dynamic sizes.
	/// (When `try_fixed` is false, only uncompressed and dynamic sizes are
	/// calculated and compared.)
	pub(crate) fn block_size_auto(self, try_fixed: bool) -> Result<NonZeroU32, ZopfliError> {
		// Take the smaller of the uncompressed and dynamic costs.
		let cost = NonZeroU32::min(
			self.block_size_uncompressed()?,
			self.block_size_dynamic()?,
		);

		// Counter-intuitively, we'll usually get better block-splitting decisions
		// by ignoring fixed costs entirely unless the store is really small. This
		// condition is also necessary to maintain parity with the original zopfli.
		if try_fixed {
			let cost2 = self.block_size_fixed();
			if cost2 < cost { return Ok(cost2); }
		}

		Ok(cost)
	}

	/// # Calculate Block Size (Dynamic).
	///
	/// This calculation is… a lot. See the `rle` module for more information.
	pub(crate) fn block_size_dynamic(self) -> Result<NonZeroU32, ZopfliError> {
		DynamicLengths::new(self).map(DynamicLengths::take_size)
	}

	/// # Calculate Block Size (Fixed).
	pub(crate) fn block_size_fixed(self) -> NonZeroU32 {
		// Loop the store if we have data to loop.
		let size = self.entries.iter()
			.map(LZ77StoreEntry::fixed_cost)
			.sum::<u32>();

		NZ07.saturating_add(size) // FIXED_TREE_LL[256]
	}

	/// # Calculate Block Size (Uncompressed).
	pub(crate) fn block_size_uncompressed(self) -> Result<NonZeroU32, ZopfliError> {
		let blocksize = self.byte_range()?.len32();

		// Uncompressed blocks are split at u16::MAX.
		let chunks = blocksize.get().div_ceil(u32::from(u16::MAX));

		Ok(NZ08.saturating_mul(blocksize).saturating_add(chunks * 40))
	}
}



/// # Ranged Store Splits.
///
/// This iterator yields all non-empty split pairs of a ranged store.
pub(crate) struct LZ77StoreRangeSplits<'a> {
	/// # Entries.
	entries: &'a [LZ77StoreEntry],

	/// # Splits.
	splits: Range<usize>,
}

impl<'a> Iterator for LZ77StoreRangeSplits<'a> {
	type Item = (LZ77StoreRange<'a>, LZ77StoreRange<'a>);

	fn next(&mut self) -> Option<Self::Item> {
		let mid = self.splits.next()?;
		// The split range is 1..entries.len() and we already verified that
		// 1 < entries.len() at construction, so this should never fail.
		let (a, b) = self.entries.split_at_checked(mid)?;
		Some((
			LZ77StoreRange { entries: a },
			LZ77StoreRange { entries: b },
		))
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.splits.len();
		(len, Some(len))
	}
}

impl ExactSizeIterator for LZ77StoreRangeSplits<'_> {
	fn len(&self) -> usize { self.splits.len() }
}



/// # Chunked Ranged Store Splits.
///
/// This iterator yields nine evenly-divided, non-empty split pairs of a ranged
/// store, used for minimum-cost-finding.
pub(crate) struct LZ77StoreRangeSplitsChunked<'a> {
	/// # Entries.
	entries: &'a [LZ77StoreEntry],

	/// # Chunk Size.
	chunk: NonZeroUsize,

	/// # Start Index.
	start: usize,

	/// # End Index.
	end: usize,

	/// # Current Index.
	pos: usize,
}

impl<'a> LZ77StoreRangeSplitsChunked<'a> {
	/// # Minimum Split Length.
	pub(crate) const SPLIT_MIN: usize = 10;

	/// # Total Splits.
	pub(crate) const SPLITS: usize = Self::SPLIT_MIN - 1;

	/// # New Instance.
	///
	/// This returns a new chunked split iterator unless the store is too small
	/// to split.
	pub(crate) const fn new(entries: &'a [LZ77StoreEntry]) -> Option<Self> {
		let end = entries.len();
		if let Some(chunk) = Self::chunk_size(1, end) {
			Some(Self {
				entries,
				chunk,
				start: 1,
				end,
				pos: 0,
			})
		}
		else { None }
	}

	/// # Chunk Size.
	///
	/// Return the chunk size, if any, given the distance between `start..end`.
	const fn chunk_size(start: usize, end: usize) -> Option<NonZeroUsize> {
		if start < end {
			NonZeroUsize::new((end - start).wrapping_div(Self::SPLIT_MIN))
		}
		else { None }
	}

	/// # Position At Chunk.
	///
	/// This returns the split point for a given chunk.
	///
	/// Note: this does not enforce the chunk count; nonsensical values may
	/// be pushed out of range.
	const fn pos_at(&self, n: usize) -> usize {
		self.start + n * self.chunk.get()
	}

	#[expect(unsafe_code, reason = "Pos is non-zero.")]
	/// # Reset/Resplit the Range.
	///
	/// This builds a new range around chunk number `n` — the position of
	/// that chunk becomes the start, the end becomes +2 chunks — and so long
	/// as the new range is iterable, resets the counters so this can be looped
	/// anew.
	///
	/// Regardless, the mid point between start and end is returned.
	pub(crate) fn reset(&mut self, n: usize) -> NonZeroUsize {
		// Find the midpoint first; the new start and end are relative to it.
		// Safety: chunk and (n+1) are both non-zero.
		let mid = unsafe { NonZeroUsize::new_unchecked(self.pos_at(n + 1)) };

		// Tweak the ranges.
		if 0 != n { self.start = mid.get() - self.chunk.get(); };
		if n + 1 < Self::SPLITS { self.end = mid.get() + self.chunk.get(); }

		// If we're still chunkable, reset for another round!
		if let Some(chunk) = Self::chunk_size(self.start, self.end) {
			self.chunk = chunk;
			self.pos = 0;
		}

		mid
	}
}

impl<'a> Iterator for LZ77StoreRangeSplitsChunked<'a> {
	type Item = (usize, LZ77StoreRange<'a>, LZ77StoreRange<'a>);

	fn next(&mut self) -> Option<Self::Item> {
		let idx = self.pos;
		if idx < Self::SPLITS {
			self.pos = idx + 1;
			let mid = self.pos_at(self.pos);

			// We verified the chunk size and entry count at construction so
			// this shouldn't fail.
			let (a, b) = self.entries.split_at_checked(mid)?;
			Some((
				idx,
				LZ77StoreRange { entries: a },
				LZ77StoreRange { entries: b },
			))
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl ExactSizeIterator for LZ77StoreRangeSplitsChunked<'_> {
	fn len(&self) -> usize { Self::SPLITS.saturating_sub(self.pos) }
}



#[derive(Clone, Copy)]
/// # LZ77 Store Entry.
///
/// This struct holds all of the relevant details for a given entry, including
/// its index in the original uncompressed chunk, the length and distance pair,
/// and the corresponding length and distance symbols.
pub(crate) struct LZ77StoreEntry {
	/// # Original (Uncompressed) Index.
	pub(crate) pos: usize,

	/// # Litlen Symbol.
	pub(crate) litlen: LitLen,

	/// # Distance.
	pub(crate) dist: i16,

	/// # Length Symbol or Litlen.
	pub(crate) ll_symbol: Lsym,

	/// # Distance Symbol.
	pub(crate) d_symbol: Dsym,
}

impl LZ77StoreEntry {
	#[expect(
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
		reason = "False positive.",
	)]
	/// # New.
	const fn new(litlen: LitLen, dist: u16, pos: usize) -> Self {
		debug_assert!(dist < 32_768, "BUG: distance exceeds the WINDOW_SIZE!");

		// Using the signed type helps the compiler understand the upper
		// range fits ZOPFLI_WINDOW_MAX. Impossibly large values would also
		// get neatly tucked away in negative-land and ignored, but that'd be
		// impossible!
		let dist = dist as i16;
		let (ll_symbol, d_symbol) =
			if 0 < dist {(
				LENGTH_SYMBOLS[litlen as usize],
				DISTANCE_SYMBOLS[dist as usize],
			)}
			else { (Lsym::from_litlen(litlen), Dsym::D00) };

		Self {
			pos,
			litlen,
			dist,
			ll_symbol,
			d_symbol,
		}
	}

	/// # Fixed Cost.
	///
	/// Note: these values all fit comfortably within `u8`, but we never just
	/// want one cost, so the result is widened to `u32` to simplify
	/// `LZ77StoreRange::block_size_fixed`'s efforts.
	const fn fixed_cost(&self) -> u32 {
		let base = FIXED_TREE_LL[self.ll_symbol as usize] as u8;
		let extra =
			if 0 < self.dist {
				LENGTH_SYMBOL_BITS[self.litlen as usize] +
				DISTANCE_BITS[self.d_symbol as usize] +
				5 // FIXED_TREE_D.
			}
			else { 0 };
		(base + extra) as u32
	}

	/// # Length.
	///
	/// If the distance is zero, 1, otherwise the litlen.
	pub(crate) const fn length(&self) -> LitLen {
		if 0 < self.dist { self.litlen }
		else { LitLen::L001 }
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_fixed_tree_256() {
		// Our use of this particular index is hardcoded for simplicity; let's
		// triple-check we chose correctly!
		assert_eq!(FIXED_TREE_LL[256] as u32, NZ07.get());
	}

	#[test]
	fn t_fixed_tree_d5() {
		// Our use of this particular index is hardcoded for simplicity; let's
		// triple-check we chose correctly!
		assert!(super::super::FIXED_TREE_D.iter().all(|&d| d as u32 == 5));
	}

	#[test]
	fn t_ranged_splits() {
		/// # Poor Man's Equal Impl.
		///
		/// Most of these types do not implement (or need) `Eq`, but since
		/// we're only setting `pos` and `dist` uniquely here anyway, we can
		/// limit matching to those two.
		const fn entry_eq((a, b): (&LZ77StoreEntry, &LZ77StoreEntry)) -> bool {
			a.pos == b.pos && a.dist == b.dist
		}

		// Generate an entry with the given pos and dist.
		macro_rules! entry {
			($i:literal) => (
				LZ77StoreEntry {
					pos: $i,
					litlen: LitLen::L000,
					dist: $i,
					ll_symbol: Lsym::L000,
					d_symbol: Dsym::D00,
				}
			);
		}

		// These entries are nonsensical, but all we're looking to do is check
		// that splits are happening in the right place, so they only really
		// need to be unique from one another.
		let arr: &[LZ77StoreEntry] = &[
			entry!(0),
			entry!(1),
			entry!(2),
			entry!(3),
			entry!(4),
			entry!(5),
		];
		let store = LZ77StoreRange { entries: arr };

		// Do the splits.
		let mut splits = store.splits().expect("failed to split store");
		for i in 1..arr.len() {
			assert_eq!(splits.len(), arr.len() - i);
			let (a, b) = splits.next().expect("expected next split");
			let c = &arr[..i]; // Expected A.
			let d = &arr[i..]; // Expected B.

			assert_eq!(a.len().get(), a.entries.len());
			assert_eq!(a.entries.len(), c.len());
			assert!(a.entries.iter().zip(c.iter()).all(entry_eq));

			assert_eq!(b.len().get(), b.entries.len());
			assert_eq!(b.entries.len(), d.len());
			assert!(b.entries.iter().zip(d.iter()).all(entry_eq));
		}

		// We should be empty.
		assert_eq!(splits.len(), 0);
		assert!(splits.next().is_none());
	}
}
