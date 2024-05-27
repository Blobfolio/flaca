/*!
# Flaca: Zopflipng LZ77 Store.

This module defines the LZ77 store structure.
*/

use std::{
	mem::ManuallyDrop,
	ops::{
		Deref,
		DerefMut,
	},
	sync::Mutex,
};
use super::{
	DISTANCE_SYMBOLS,
	Dsym,
	LENGTH_SYMBOLS_BITS_VALUES,
	LitLen,
	Lsym,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
};



/// # Shared `LZ77Store` Pool.
///
/// Each `deflate_part` run can use as many as three of these; we might as well
/// reuse the objects to cut down on the number of allocations being made.
static POOL: Pool = Pool::new();



#[derive(Clone)]
/// # LZ77 Data Store.
pub(crate) struct LZ77Store {
	pub(crate) entries: Vec<LZ77StoreEntry>,
	pub(crate) ll_counts: Vec<[u32; ZOPFLI_NUM_LL]>,
	pub(crate) d_counts: Vec<[u32; ZOPFLI_NUM_D]>,
}

impl LZ77Store {
	#[allow(clippy::new_ret_no_self)]
	/// # New.
	pub(crate) fn new() -> Swimmer { POOL.get() }

	/// # Internal New.
	const fn _new() -> Self {
		Self {
			entries: Vec::new(),
			ll_counts: Vec::new(),
			d_counts: Vec::new(),
		}
	}

	/// # Append Entries.
	///
	/// This appends the entries from `other` to `self` en masse, kind of like
	/// a push-many.
	pub(crate) fn append(&mut self, other: &Self) {
		self.entries.reserve_exact(other.entries.len());
		for &entry in &other.entries { self.push_entry(entry); }
	}

	/// # Clear.
	pub(crate) fn clear(&mut self) {
		self.entries.truncate(0);
		self.ll_counts.truncate(0);
		self.d_counts.truncate(0);
	}

	/// # Push Values.
	pub(crate) fn push(&mut self, litlen: u16, dist: u16, pos: usize) -> Result<(), ZopfliError> {
		let e = LZ77StoreEntry::new(litlen, dist, pos)?;
		self.push_entry(e);
		Ok(())
	}

	#[allow(unsafe_code)]
	/// # Last Counts.
	///
	/// Return the last (current) length and distance count chunks, resizing as
	/// needed.
	fn last_counts(&mut self) -> (&mut [u32; ZOPFLI_NUM_LL], &mut [u32; ZOPFLI_NUM_D]) {
		/// # (Maybe) Wrap Count Chunks.
		///
		/// Resize the chunks if needed and return the final length.
		fn wrap_chunk<const SIZE: usize>(set: &mut Vec<[u32; SIZE]>, pos: usize) -> usize {
			let len = set.len();

			// If the set is empty, set it up with a zeroed count chunk.
			if len == 0 {
				set.push([0; SIZE]);
				1
			}
			// If the position has wrapped back around SIZE, it's time for a
			// new chunk, initialized with the previous chunk's tallies.
			else if pos % SIZE == 0 {
				set.push(set[len - 1]);
				len + 1
			}
			// Otherwise we're good.
			else { len }
		}

		let pos = self.entries.len();
		let d_len = wrap_chunk(&mut self.d_counts, pos);
		let ll_len = wrap_chunk(&mut self.ll_counts, pos);

		// Safety: neither can be empty at this point.
		unsafe {(
			self.ll_counts.get_unchecked_mut(ll_len - 1),
			self.d_counts.get_unchecked_mut(d_len - 1),
		)}
	}

	/// # Push Entry.
	fn push_entry(&mut self, entry: LZ77StoreEntry) {
		let (ll_counts, d_counts) = self.last_counts();
		entry.add_counts(ll_counts, d_counts);
		self.entries.push(entry);
	}

	/// # Replace Store.
	///
	/// Replace the current content with some other store's content.
	pub(crate) fn replace(&mut self, other: &Self) {
		self.entries.truncate(0);
		self.entries.extend_from_slice(&other.entries);

		self.ll_counts.truncate(0);
		self.ll_counts.extend_from_slice(&other.ll_counts);

		self.d_counts.truncate(0);
		self.d_counts.extend_from_slice(&other.d_counts);
	}
}

impl LZ77Store {
	/// # Length.
	pub(crate) fn len(&self) -> usize { self.entries.len() }

	/// # Histogram.
	pub(crate) fn histogram(&self, lstart: usize, lend: usize)
	-> Result<([u32; ZOPFLI_NUM_LL], [u32; ZOPFLI_NUM_D]), ZopfliError> {
		// Count the symbols directly.
		if lstart + ZOPFLI_NUM_LL * 3 > lend {
			let mut ll_counts = [0_u32; ZOPFLI_NUM_LL];
			let mut d_counts = [0_u32; ZOPFLI_NUM_D];

			let entries = self.entries.get(lstart..lend).ok_or(zopfli_error!())?;
			for e in entries {
				e.add_counts(&mut ll_counts, &mut d_counts);
			}

			Ok((ll_counts, d_counts))
		}
		// Subtract the cumulative histograms at the start from the end to get the
		// one for this range.
		else {
			let (mut ll_counts, mut d_counts) = self._histogram(lend - 1)?;
			if 0 < lstart {
				self._histogram_sub(lstart - 1, &mut ll_counts, &mut d_counts)?;
			}

			Ok((ll_counts, d_counts))
		}
	}

	/// # Histogram at Position.
	fn _histogram(&self, pos: usize)
	-> Result<([u32; ZOPFLI_NUM_LL], [u32; ZOPFLI_NUM_D]), ZopfliError> {
		// The relative chunked positions.
		let ll_idx = pos.wrapping_div(ZOPFLI_NUM_LL);
		let d_idx = pos.wrapping_div(ZOPFLI_NUM_D);
		let ll_end = (ll_idx + 1) * ZOPFLI_NUM_LL;
		let d_end = (d_idx + 1) * ZOPFLI_NUM_D;

		// Start by copying the counts directly from the nearest chunk.
		if self.ll_counts.len() <= ll_idx || self.d_counts.len() <= d_idx {
			return Err(zopfli_error!());
		}
		let mut ll_counts: [u32; ZOPFLI_NUM_LL] = self.ll_counts[ll_idx];
		let mut d_counts: [u32; ZOPFLI_NUM_D] = self.d_counts[d_idx];

		// Subtract the symbol occurences between (pos+1) and the end of the
		// available data for the chunk.
		for (i, e) in self.entries.iter().enumerate().take(ll_end).skip(pos + 1) {
			ll_counts[e.ll_symbol as usize] -= 1;
			if i < d_end && 0 < e.dist {
				d_counts[e.d_symbol as usize] -= 1;
			}
		}

		// We have our answer!
		Ok((ll_counts, d_counts))
	}

	/// # Subtract Histogram.
	fn _histogram_sub(
		&self,
		pos: usize,
		ll_counts: &mut [u32; ZOPFLI_NUM_LL],
		d_counts: &mut [u32; ZOPFLI_NUM_D],
	) -> Result<(), ZopfliError> {
		// The relative chunked positions.
		let ll_idx = pos.wrapping_div(ZOPFLI_NUM_LL);
		let d_idx = pos.wrapping_div(ZOPFLI_NUM_D);

		// Start by copying the counts directly from the nearest chunk.
		let (ll_old, d_old) = self.ll_counts.get(ll_idx)
			.zip(self.d_counts.get(d_idx))
			.ok_or(zopfli_error!())?;

		// We're ultimately looking for `a -= (b_count - b_sym)`. Let's start
		// by adding — minus-minus is plus — the symbols.
		let ll_end = (ll_idx + 1) * ZOPFLI_NUM_LL;
		let d_end = (d_idx + 1) * ZOPFLI_NUM_D;
		for (i, e) in self.entries.iter().enumerate().take(ll_end).skip(pos + 1) {
			ll_counts[e.ll_symbol as usize] += 1;
			if i < d_end && 0 < e.dist {
				d_counts[e.d_symbol as usize] += 1;
			}
		}

		// To finish it off, we just need to subtract the counts. Slicing the
		// store side serves as both a sanity check and a potential compiler
		// size hint.
		for (a, b) in ll_counts.iter_mut().zip(ll_old) { *a -= b; }
		for (a, b) in d_counts.iter_mut().zip(d_old) { *a -= b; }

		Ok(())
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
		unsafe_code,
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
	)]
	#[inline(never)]
	/// # New.
	const fn new(litlen: u16, dist: u16, pos: usize) -> Result<Self, ZopfliError> {
		if litlen < 259 && dist < 32_768 {
			// Using the signed type helps the compiler understand the upper
			// range fits ZOPFLI_WINDOW_MAX.
			let dist = dist as i16;
			let (ll_symbol, d_symbol) =
				if dist <= 0 {
					// Safety: the maximum Lsym is 285.
					(unsafe { std::mem::transmute::<u16, Lsym>(litlen) }, Dsym::D00)
				}
				else {(
					LENGTH_SYMBOLS_BITS_VALUES[litlen as usize].0,
					DISTANCE_SYMBOLS[dist as usize],
				)};

			Ok(Self {
				pos,
				litlen: unsafe { std::mem::transmute::<u16, LitLen>(litlen) },
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

	/// # Add Symbol Counts.
	fn add_counts(
		&self,
		ll_counts: &mut [u32; ZOPFLI_NUM_LL],
		d_counts: &mut [u32; ZOPFLI_NUM_D],
	) {
		ll_counts[self.ll_symbol as usize] += 1;
		if 0 < self.dist {
			d_counts[self.d_symbol as usize] += 1;
		}
	}
}



/// # Cheap Object Pool.
///
/// This is an extremely simple pop/push object pool.
struct Pool {
	inner: Mutex<Vec<LZ77Store>>,
}

impl Pool {
	/// # New!
	const fn new() -> Self {
		Self { inner: Mutex::new(Vec::new()) }
	}

	/// # Get!
	///
	/// Return a store from the cache if possible, or create one if not.
	fn get(&self) -> Swimmer {
		let store = self.inner.lock()
			.ok()
			.and_then(|mut v| v.pop())
			.unwrap_or_else(LZ77Store::_new);
		Swimmer(ManuallyDrop::new(store))
	}
}



#[repr(transparent)]
/// # Object Pool Member.
///
/// This wrapper ensures stores fetched from the pool will be automatically
/// returned when dropped (so they can be fetched again).
///
/// Note that no reset-type action is performed; it is left to the caller to
/// handle that if and when necessary.
pub(crate) struct Swimmer(ManuallyDrop<LZ77Store>);

impl Deref for Swimmer {
	type Target = LZ77Store;
	#[inline]
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for Swimmer {
	#[inline]
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Drop for Swimmer {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		if let Ok(mut ptr) = POOL.inner.lock() {
			ptr.push(unsafe { ManuallyDrop::take(&mut self.0) });
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_histogram_sub_take() {
		// In _histogram_sub(), we assume d_end <= ll_end; let's verify that
		// pattern seems to hold…
		for i in 0..=usize::from(u16::MAX) {
			let ll_start = ZOPFLI_NUM_LL * i.wrapping_div(ZOPFLI_NUM_LL);
			let d_start = ZOPFLI_NUM_D * i.wrapping_div(ZOPFLI_NUM_D);
			let ll_end = ll_start + ZOPFLI_NUM_LL;
			let d_end = d_start + ZOPFLI_NUM_D;

			assert!(d_end <= ll_end, "Failed with {i}!");
		}
	}
}
