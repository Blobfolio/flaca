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



#[allow(unsafe_code)]
/// # Seven is Non-Zero.
const NZ07: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(7) };

#[allow(unsafe_code)]
/// # Eight is Non-Zero.
const NZ08: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(8) };



#[derive(Clone)]
/// # LZ77 Data Store.
pub(crate) struct LZ77Store {
	pub(crate) entries: Vec<LZ77StoreEntry>,
}

impl LZ77Store {
	/// # New.
	pub(crate) const fn new() -> Self {
		Self { entries: Vec::new() }
	}

	/// # Ranged.
	pub(crate) fn ranged(&self, rng: ZopfliRange) -> Result<LZ77StoreRange, ZopfliError> {
		let entries = self.entries.get(rng.rng()).ok_or(zopfli_error!())?;
		Ok(LZ77StoreRange { entries })
	}

	/// # Ranged.
	pub(crate) fn ranged_full(&self) -> Result<LZ77StoreRange, ZopfliError> {
		let entries = self.entries.as_slice();
		if entries.is_empty() || ZOPFLI_MASTER_BLOCK_SIZE < entries.len() {
			Err(zopfli_error!())
		}
		else { Ok(LZ77StoreRange { entries }) }
	}

	/// # Clear.
	pub(crate) fn clear(&mut self) { self.entries.truncate(0); }

	/// # Push Values.
	pub(crate) fn push(&mut self, litlen: LitLen, dist: u16, pos: usize) {
		self.push_entry(LZ77StoreEntry::new(litlen, dist, pos));
	}

	/// # Push Entry.
	fn push_entry(&mut self, entry: LZ77StoreEntry) { self.entries.push(entry); }

	/// # Replace Store.
	///
	/// Replace the current content with some other store's content.
	pub(crate) fn replace(&mut self, other: &Self) {
		self.entries.truncate(0);
		self.entries.extend_from_slice(&other.entries);
	}

	/// # Steal/Append Entries.
	///
	/// Drain the entires from other and append them to self.
	pub(crate) fn steal_entries(&mut self, other: &mut Self) {
		self.entries.append(&mut other.entries);
	}
}

impl LZ77Store {
	/// # Length.
	pub(crate) fn len(&self) -> usize { self.entries.len() }
}



#[repr(transparent)]
#[derive(Clone, Copy)]
/// # Ranged LZ77 Data Store.
///
/// Same as `LZ77Store`, but immutable and non-empty, allowing for more const-
/// type operations, `Copy`, etc.
pub(crate) struct LZ77StoreRange<'a> {
	pub(crate) entries: &'a [LZ77StoreEntry],
}

impl<'a> LZ77StoreRange<'a> {
	/// # Uncompressed Range.
	///
	/// Return the original uncompressed range used to build this store.
	pub(crate) const fn byte_range(self) -> Result<ZopfliRange, ZopfliError> {
		// Safety: ranged stores are never empty.
		let len = self.entries.len();
		if 0 == len { crate::unreachable(); }

		let first = self.entries[0];
		let last = self.entries[len - 1];
		ZopfliRange::new(first.pos, last.length() as usize + last.pos)
	}

	/// # Histogram.
	pub(crate) fn histogram(self) -> (ArrayLL<u32>, ArrayD<u32>) {
		let mut ll_counts = ZEROED_COUNTS_LL;
		let mut d_counts = ZEROED_COUNTS_D;

		for e in self.entries {
			ll_counts[e.ll_symbol as usize] += 1;
			if 0 < e.dist { d_counts[e.d_symbol as usize] += 1; }
		}

		(ll_counts, d_counts)
	}

	/// # Length.
	pub(crate) const fn len(self) -> NonZeroUsize {
		#[allow(unsafe_code)]
		// Safety: we verified the store is non-empty at construction.
		unsafe { NonZeroUsize::new_unchecked(self.entries.len()) }
	}

	#[allow(unsafe_code)]
	/// # Split.
	pub(crate) const fn split(self, mid: usize) -> Result<(Self, Self), ZopfliError> {
		if 0 == mid || self.entries.len() <= mid { Err(zopfli_error!()) }
		else {
			// Safety: we have checked mid is between the start and end of our
			// entries.
			let (a, b) = unsafe { self.entries.split_at_unchecked(mid) };
			Ok((Self { entries: a }, Self { entries: b }))
		}
	}

	/// # Split Iterator.
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

impl<'a> LZ77StoreRange<'a> {
	/// # Calculate Block Size (Auto).
	///
	/// Return the smallest of the uncompressed, fixed, and dynamic sizes.
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
	pub(crate) fn block_size_dynamic(self) -> Result<NonZeroU32, ZopfliError> {
		super::get_dynamic_lengths(self).map(|(_, size, _, _)| size)
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
	entries: &'a [LZ77StoreEntry],
	splits: Range<usize>,
}

impl<'a> Iterator for LZ77StoreRangeSplits<'a> {
	type Item = (LZ77StoreRange<'a>, LZ77StoreRange<'a>);

	#[allow(unsafe_code)]
	fn next(&mut self) -> Option<Self::Item> {
		let mid = self.splits.next()?;
		// Safety: we verified splits was in between the start and end points
		// of our entries.
		let (a, b) = unsafe { self.entries.split_at_unchecked(mid) };
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

impl<'a> ExactSizeIterator for LZ77StoreRangeSplits<'a> {
	fn len(&self) -> usize { self.splits.len() }
}



#[derive(Clone, Copy)]
pub(crate) struct LZ77StoreEntry {
	pub(crate) pos: usize,
	pub(crate) litlen: LitLen,
	pub(crate) dist: i16,
	pub(crate) ll_symbol: Lsym,
	pub(crate) d_symbol: Dsym,
}

impl LZ77StoreEntry {
	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
	)]
	/// # New.
	const fn new(litlen: LitLen, dist: u16, pos: usize) -> Self {
		debug_assert!(dist < 32_768);

		// Using the signed type helps the compiler understand the upper
		// range fits ZOPFLI_WINDOW_MAX and wraps (impossible) bad values to
		// boot.
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
		fn entry_eq((a, b): (&LZ77StoreEntry, &LZ77StoreEntry)) -> bool {
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
