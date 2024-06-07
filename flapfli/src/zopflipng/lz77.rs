/*!
# Flapfli: LZ77 Store.

This module defines the LZ77 store structures.
*/

use super::{
	ArrayD,
	ArrayLL,
	DISTANCE_SYMBOLS,
	Dsym,
	LENGTH_SYMBOLS_BITS_VALUES,
	LitLen,
	Lsym,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	zopfli_error,
	ZopfliError,
};



/// # Shared `LZ77Store` Pool.
///
/// Each `deflate_part` run can use as many as three of these; we might as well
/// reuse the objects to cut down on the number of allocations being made.
// static POOL: Pool = Pool::new();



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
	pub(crate) fn byte_range(
		&self,
		lstart: usize,
		lend: usize,
	) -> Result<(usize, usize), ZopfliError> {
		let slice = self.entries.as_slice();
		if lstart < lend && lend <= slice.len() {
			let instart = slice[lstart].pos;
			let e = slice[lend - 1];
			Ok((instart, e.length() as usize + e.pos))
		}
		else { Err(zopfli_error!()) }
	}

	/// # Clear.
	pub(crate) fn clear(&mut self) { self.entries.truncate(0); }

	/// # Push Values.
	pub(crate) fn push(&mut self, litlen: LitLen, dist: u16, pos: usize) -> Result<(), ZopfliError> {
		let e = LZ77StoreEntry::new(litlen, dist, pos)?;
		self.push_entry(e);
		Ok(())
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
	pub(crate) fn histogram(&self, lstart: usize, lend: usize)
	-> (ArrayLL<u32>, ArrayD<u32>) {
		let mut ll_counts = ZEROED_COUNTS_LL;
		let mut d_counts = ZEROED_COUNTS_D;

		for e in self.entries.iter().take(lend).skip(lstart) {
			ll_counts[e.ll_symbol as usize] += 1;
			if 0 < e.dist {
				d_counts[e.d_symbol as usize] += 1;
			}
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
	const fn new(litlen: LitLen, dist: u16, pos: usize) -> Result<Self, ZopfliError> {
		if dist < 32_768 {
			// Using the signed type helps the compiler understand the upper
			// range fits ZOPFLI_WINDOW_MAX.
			let dist = dist as i16;
			let (ll_symbol, d_symbol) =
				if dist <= 0 { (Lsym::from_litlen(litlen), Dsym::D00) }
				else {(
					LENGTH_SYMBOLS_BITS_VALUES[litlen as usize].0,
					DISTANCE_SYMBOLS[dist as usize],
				)};

			Ok(Self {
				pos,
				litlen,
				dist,
				ll_symbol,
				d_symbol,
			})
		}
		else { Err(zopfli_error!()) }
	}

	/// # Length.
	///
	/// If the distance is zero, 1, otherwise the litlen.
	pub(crate) const fn length(&self) -> LitLen {
		if self.dist <= 0 { LitLen::L001 }
		else { self.litlen }
	}
}
