/*!
# Flapfli: LZ77 Store.

This module defines the LZ77 store structures.
*/

use std::ops::Range;
use super::{
	ArrayD,
	ArrayLL,
	DISTANCE_SYMBOLS,
	Dsym,
	LENGTH_SYMBOLS,
	LitLen,
	Lsym,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	zopfli_error,
	ZopfliError,
};



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

	/// # Symbol Span Range.
	///
	/// Convert an LZ77 range to the start/end positions of the block.
	pub(crate) fn byte_range(&self, rng: Range<usize>) -> Result<Range<usize>, ZopfliError> {
		let slice = self.entries.as_slice();
		if rng.start < rng.end && rng.end <= slice.len() {
			let instart = slice[rng.start].pos;
			let e = slice[rng.end - 1];
			Ok(instart..e.length() as usize + e.pos)
		}
		else { Err(zopfli_error!()) }
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

	/// # Histogram.
	pub(crate) fn histogram(&self, rng: Range<usize>) -> (ArrayLL<u32>, ArrayD<u32>) {
		let mut ll_counts = ZEROED_COUNTS_LL;
		let mut d_counts = ZEROED_COUNTS_D;

		for e in self.entries.iter().take(rng.end).skip(rng.start) {
			ll_counts[e.ll_symbol as usize] += 1;
			if 0 < e.dist { d_counts[e.d_symbol as usize] += 1; }
		}

		(ll_counts, d_counts)
	}
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

	/// # Length.
	///
	/// If the distance is zero, 1, otherwise the litlen.
	pub(crate) const fn length(&self) -> LitLen {
		if 0 < self.dist { self.litlen }
		else { LitLen::L001 }
	}
}
