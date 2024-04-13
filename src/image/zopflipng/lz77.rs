/*!
# Flaca: Zopflipng LZ77 Store.

This module defines the LZ77 store structure.
*/

use super::{
	DISTANCE_SYMBOLS,
	Dsym,
	LENGTH_SYMBOLS_BITS_VALUES,
	LitLen,
	Lsym,
	ZopfliError,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
};



#[derive(Clone)]
/// # LZ77 Data Store.
pub(crate) struct LZ77Store {
	pub(crate) entries: Vec<LZ77StoreEntry>,
	pub(crate) ll_counts: Vec<usize>,
	pub(crate) d_counts: Vec<usize>,
}

impl LZ77Store {
	/// # New.
	pub(crate) const fn new() -> Self {
		Self {
			entries: Vec::new(),
			ll_counts: Vec::new(),
			d_counts: Vec::new(),
		}
	}

	/// # Append Values.
	pub(crate) fn append(&mut self, other: &Self) {
		// The count counts are weird, but we can go ahead and reserve space
		// for the entries.
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
		LZ77StoreEntry::new(litlen, dist, pos).map(|e| self.push_entry(e))
	}

	#[allow(unsafe_code)]
	/// # Push Entry.
	fn push_entry(&mut self, entry: LZ77StoreEntry) {
		let old_len = self.entries.len();
		let ll_start = ZOPFLI_NUM_LL * old_len.wrapping_div(ZOPFLI_NUM_LL);
		let d_start = ZOPFLI_NUM_D * old_len.wrapping_div(ZOPFLI_NUM_D);

		// The histograms are wrapping and cumulative, and need to be extended
		// any time we reach a new ZOPFLI_NUM_* bucket level.
		if old_len == 0 {
			self.ll_counts.resize(ZOPFLI_NUM_LL, 0);
			self.d_counts.resize(ZOPFLI_NUM_D, 0);
		}
		else {
			if old_len % ZOPFLI_NUM_LL == 0 {
				self.ll_counts.extend_from_within((old_len - ZOPFLI_NUM_LL)..old_len);
			}

			if old_len % ZOPFLI_NUM_D == 0 {
				self.d_counts.extend_from_within((old_len - ZOPFLI_NUM_D)..old_len);
			}
		}

		// If the distance is zero, we just need to bump the litlen count.
		if entry.dist <= 0 {
			// Safety: the counts were just resized a few lines back.
			unsafe {
				*self.ll_counts.get_unchecked_mut(ll_start + entry.litlen as usize) += 1;
			}
		}
		// If it is non-zero, we need to set the correct symbols and bump both
		// counts.
		else {
			// Safety: the counts were just resized a few lines back.
			unsafe {
				*self.ll_counts.get_unchecked_mut(ll_start + entry.ll_symbol as usize) += 1;
				*self.d_counts.get_unchecked_mut(d_start + entry.d_symbol as usize) += 1;
			}
		}

		// Don't forget to push the entry!
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
	-> Result<([usize; ZOPFLI_NUM_LL], [usize; ZOPFLI_NUM_D]), ZopfliError> {
		// Count the symbols directly.
		if lstart + ZOPFLI_NUM_LL * 3 > lend {
			let mut ll_counts = [0_usize; ZOPFLI_NUM_LL];
			let mut d_counts = [0_usize; ZOPFLI_NUM_D];

			for e in &self.entries[lstart..lend] {
				ll_counts[e.ll_symbol as usize] += 1;
				if 0 < e.dist {
					d_counts[e.d_symbol as usize] += 1;
				}
			}

			Ok((ll_counts, d_counts))
		}
		// Subtract the cumulative histograms at the start from the end to get the
		// one for this range.
		else {
			let (mut ll_counts, mut d_counts) = self._histogram(lend - 1)?;
			if 0 < lstart {
				self._histogram_sub(lstart - 1, &mut ll_counts, &mut d_counts);
			}

			Ok((ll_counts, d_counts))
		}
	}

	/// # Histogram at Position.
	fn _histogram(&self, pos: usize)
	-> Result<([usize; ZOPFLI_NUM_LL], [usize; ZOPFLI_NUM_D]), ZopfliError> {
		// The relative chunked positions.
		let ll_start = ZOPFLI_NUM_LL * pos.wrapping_div(ZOPFLI_NUM_LL);
		let d_start = ZOPFLI_NUM_D * pos.wrapping_div(ZOPFLI_NUM_D);
		let ll_end = ll_start + ZOPFLI_NUM_LL;
		let d_end = d_start + ZOPFLI_NUM_D;

		// Start by copying the counts directly from the nearest chunk.
		let mut ll_counts: [usize; ZOPFLI_NUM_LL] = self.ll_counts.get(ll_start..ll_end)
			.and_then(|c| c.try_into().ok())
			.ok_or(ZopfliError::HistogramRange)?;
		let mut d_counts: [usize; ZOPFLI_NUM_D] = self.d_counts.get(d_start..d_end)
			.and_then(|c| c.try_into().ok())
			.ok_or(ZopfliError::HistogramRange)?;

		// Subtract the symbol occurences between (pos+1) and the end of the
		// chunks.
		for (i, e) in self.entries.iter().enumerate().take(ll_end.max(d_end)).skip(pos + 1) {
			if i < ll_end {
				ll_counts[e.ll_symbol as usize] -= 1;
			}
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
		ll_counts: &mut [usize; ZOPFLI_NUM_LL],
		d_counts: &mut [usize; ZOPFLI_NUM_D],
	) {
		// The relative chunked positions.
		let ll_start = ZOPFLI_NUM_LL * pos.wrapping_div(ZOPFLI_NUM_LL);
		let d_start = ZOPFLI_NUM_D * pos.wrapping_div(ZOPFLI_NUM_D);
		let ll_end = ll_start + ZOPFLI_NUM_LL;
		let d_end = d_start + ZOPFLI_NUM_D;

		// We ultimately need to subtract (start_counts - start_symbols) from
		// the end_counts. We can avoid intermediate storage by rearranging
		// the formula so that the start_symbols get _added_ to the end_counts
		// directly.
		for (i, e) in self.entries.iter().enumerate().take(ll_end.max(d_end)).skip(pos + 1) {
			if i < ll_end {
				ll_counts[e.ll_symbol as usize] += 1;
			}
			if i < d_end && 0 < e.dist {
				d_counts[e.d_symbol as usize] += 1;
			}
		}

		// Now we just need to subtract the start_counts, et voilà, we have our
		// desired middle stats!
		for (a, b) in ll_counts.iter_mut().zip(self.ll_counts.iter().skip(ll_start)) {
			*a -= b;
		}
		for (a, b) in d_counts.iter_mut().zip(self.d_counts.iter().skip(d_start)) {
			*a -= b;
		}
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
	/// # New.
	const fn new(litlen: u16, dist: u16, pos: usize) -> Result<Self, ZopfliError> {
		if litlen >= 259 { return Err(ZopfliError::LitLen(litlen)); }
		debug_assert!(dist < 32_768);

		// Using the signed type helps the compiler understand the upper
		// range fits ZOPFLI_WINDOW_MAX.
		let dist = dist as i16;
		let (ll_symbol, d_symbol) =
			if dist <= 0 {
				// Safety: the maximum Lsym is 285.
				(unsafe { std::mem::transmute(litlen) }, Dsym::D00)
			}
			else {(
				LENGTH_SYMBOLS_BITS_VALUES[litlen as usize].0,
				DISTANCE_SYMBOLS[dist as usize],
			)};

		Ok(Self {
			pos,
			litlen: unsafe { std::mem::transmute(litlen) },
			dist,
			ll_symbol,
			d_symbol,
		})
	}

	/// # Length.
	///
	/// If the distance is zero, 1, otherwise the litlen.
	pub(crate) const fn length(&self) -> LitLen {
		if self.dist <= 0 { LitLen::L001 }
		else { self.litlen }
	}
}
