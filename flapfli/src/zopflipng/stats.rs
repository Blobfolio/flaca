/*!
# Flapfli: Squeeze Stats.

This module defines the squeeze stats structure and its companion PRNG.
*/

use std::num::NonZeroU32;
use super::{
	ArrayD,
	ArrayLL,
	LZ77Store,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
};



#[expect(clippy::missing_docs_in_private_items, reason = "Unimportant.")]
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
	const fn randomize(&mut self) -> u32 {
		self.m_z = 36_969 * (self.m_z & 65_535) + (self.m_z >> 16);
		self.m_w = 18_000 * (self.m_w & 65_535) + (self.m_w >> 16);
		(self.m_z << 16).wrapping_add(self.m_w)
	}
}



#[derive(Clone, Copy)]
/// # Symbol Stats.
///
/// This hols the length and distance symbols and costs for a given block.
/// data which can be used to improve compression on subsequent passes.
pub(crate) struct SymbolStats {
	/// # Litlen Symbol Counts.
	ll_counts: ArrayLL<u32>,

	/// # Distance Symbol Counts.
	d_counts:  ArrayD<u32>,

	/// # Litlen Symbols.
	pub(crate) ll_symbols: ArrayLL<f64>,

	/// # Distance Symbols.
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
	/// This calculates the "entropy" of the `ll_counts` and `d_counts` — a
	/// fancy way of saying the difference between the log2 of everything and
	/// the log2 of self — storing the results in the corresponding symbol
	/// arrays.
	///
	/// Note: the symbols are only valid for the _current_ counts, but they
	/// don't need to be rebuilt after each and every little change because
	/// they're only ever referenced during `ZopfliState::optimal_run` passes;
	/// so long as they're (re)crunched before that method is called, life is
	/// grand.
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

		// Lengths second. (Note to future self: ll_counts[256] is always 1, so
		// the sum should never be zero, and the log2 should always be normal.)
		let log2sum = NonZeroU32::new(self.ll_counts.iter().copied().sum::<u32>())
			.map_or(0.0, |sum| f64::from(sum.get()).log2());
		self.ll_symbols.fill(log2sum);
		for (c, b) in self.ll_counts.iter().copied().zip(&mut self.ll_symbols) {
			if c != 0 { *b -= f64::from(c).log2(); }
		}
	}

	/// # Load Statistics.
	///
	/// This updates the `ll_counts` and `d_counts` stats using the data from the
	/// `LZ77Store` store.
	///
	/// Note: this does _not_ rebuild the symbol tables.
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
	///
	/// For this to work properly, a single `RanState` must be used for all
	/// iterations, and because shuffling advances the `RanState`, litlens must
	/// be processed before distances.
	///
	/// Yeah… this is super weird. Haha.
	///
	/// Note: this does _not_ rebuild the symbol tables.
	pub(crate) fn randomize(&mut self, state: &mut RanState) {
		/// # Shuffle Counts.
		fn shuffle_counts<const N: usize>(counts: &mut [u32; N], state: &mut RanState) {
			const {
				assert!(
					N == ZOPFLI_NUM_D || N == ZOPFLI_NUM_LL,
					"BUG: counts must have a length of 32 or 288.",
				);
			}
			for i in const { 0..N } {
				// TODO: use is_multiple_of once stable
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
	/// (Halving creates a sort of weighted average, useful once a few
	/// iterations have occurred.)
	///
	/// Note: this does _not_ rebuild the symbols.
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
	#[expect(
		clippy::cast_precision_loss,
		clippy::float_cmp,
		reason = "It is what it is.",
	)]
	fn t_d_log2() {
		// Make sure we precomputed the 32.log2() correctly!
		assert_eq!((ZOPFLI_NUM_D as f64).log2(), 5.0);
	}
}
