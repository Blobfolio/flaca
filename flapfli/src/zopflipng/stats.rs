/*!
# Flapfli: Squeeze Stats.

This module defines the squeeze stats structure and its companion PRNG.
*/

use super::{
	ArrayD,
	ArrayLL,
	LZ77Store,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
};



#[derive(Clone, Copy)]
/// # Randomness.
///
/// This struct is only used to cheaply (and predictably) shuffle stat
/// frequencies.
pub(crate) struct RanState {
	m_w: u32,
	m_z: u32,
}

impl RanState {
	/// # New Instance.
	pub(crate) const fn new() -> Self {
		Self {
			m_w: 1,
			m_z: 2,
		}
	}

	/// # Generate Random Number.
	///
	/// A simple, repeatable [MWC PRNG](https://en.wikipedia.org/wiki/Multiply-with-carry_pseudorandom_number_generator),
	/// used to shuffle frequencies between runs.
	fn randomize(&mut self) -> u32 {
		self.m_z = 36_969 * (self.m_z & 65_535) + (self.m_z >> 16);
		self.m_w = 18_000 * (self.m_w & 65_535) + (self.m_w >> 16);
		(self.m_z << 16).wrapping_add(self.m_w)
	}
}



#[derive(Clone, Copy)]
/// # Symbol Stats.
///
/// This holds the length and distance symbols and costs for a given block,
/// data that can be used to improve compression on subsequent passes.
pub(crate) struct SymbolStats {
	ll_counts: ArrayLL<u32>,
	d_counts:  ArrayD<u32>,

	pub(crate) ll_symbols: ArrayLL<f64>,
	pub(crate) d_symbols:  ArrayD<f64>,
}

impl SymbolStats {
	/// # New Instance.
	pub(crate) const fn new() -> Self {
		Self {
			ll_counts:  ZEROED_COUNTS_LL,
			d_counts:   ZEROED_COUNTS_D,

			ll_symbols: [0.0; ZOPFLI_NUM_LL],
			d_symbols:  [0.0; ZOPFLI_NUM_D],
		}
	}
}

impl SymbolStats {
	/// # Crunch Symbols.
	///
	/// This calculates the "entropy" of the `ll_counts` and `d_counts`, storing the
	/// results in the corresponding symbols arrays.
	pub(crate) fn crunch(&mut self) {
		// Distances first.
		let sum = self.d_counts.iter().copied().sum::<u32>();
		let log2sum =
			if sum == 0 { 5.0 } // 32.log2()
			else { f64::from(sum).log2() };
		self.d_symbols.fill(log2sum);
		for (c, b) in self.d_counts.iter().copied().zip(&mut self.d_symbols) {
			if c != 0 { *b -= f64::from(c).log2(); }
		}

		// Lengths second.
		let sum = self.ll_counts.iter().copied().sum::<u32>();
		#[allow(unsafe_code)]
		if sum == 0 {
			// Safety: ll_counts[256] is always 1 — (re)load_store and
			// randomize both force it — so this sum will always be nonzero.
			unsafe { core::hint::unreachable_unchecked(); }
		}
		let log2sum = f64::from(sum).log2();
		self.ll_symbols.fill(log2sum);
		for (c, b) in self.ll_counts.iter().copied().zip(&mut self.ll_symbols) {
			if c != 0 { *b -= f64::from(c).log2(); }
		}
	}

	/// # Load Statistics.
	///
	/// This updates the `ll_counts` and `d_counts` stats using the data from the
	/// `ZopfliLZ77Store` store, then crunches the results.
	pub(crate) fn load_store(&mut self, store: &LZ77Store) {
		for e in &store.entries {
			self.ll_counts[e.ll_symbol as usize] += 1;
			if 0 < e.dist { self.d_counts[e.d_symbol as usize] += 1; }
		}

		// Set the end symbol and crunch.
		self.ll_counts[256] = 1;
	}

	/// # Randomize Stat Frequencies.
	///
	/// This randomizes the stat frequencies to allow things to maybe turn out
	/// different on subsequent squeeze passes.
	pub(crate) fn randomize(&mut self, state: &mut RanState) {
		fn shuffle_counts<const N: usize>(counts: &mut [u32; N], state: &mut RanState) {
			const { assert!(N == ZOPFLI_NUM_D || N == ZOPFLI_NUM_LL); }
			for i in const { 0..N } {
				if (state.randomize() >> 4) % 3 == 0 {
					let index = state.randomize() as usize % N;
					counts[i] = counts[index];
				}
			}
		}
		shuffle_counts(&mut self.ll_counts, state); // Lengths need to go first.
		shuffle_counts(&mut self.d_counts, state);

		// Set the end symbol.
		self.ll_counts[256] = 1;
	}

	/// # Reload Store.
	///
	/// Like `SymbolStats::load_store`, but reset or halve the counts first.
	/// (Halving creates a sort of weighted average, useful after a few
	/// iterations have passed.)
	pub(crate) fn reload_store(&mut self, store: &LZ77Store, weighted: bool) {
		if weighted {
			for c in &mut self.d_counts { *c /= 2; }
			for c in &mut self.ll_counts { *c /= 2; }
		}
		else {
			self.d_counts.fill(0);
			self.ll_counts.fill(0);
		}

		self.load_store(store);
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_d_log2() {
		// Make sure we precomputed the 32.log2() correctly!
		assert_eq!((ZOPFLI_NUM_D as f64).log2(), 5.0);
	}
}
