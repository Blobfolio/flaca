/*!
# Flapfli: Squeeze Stats.

This module defines the squeeze stats structure and its companion PRNG.
*/

use super::{
	LZ77Store,
	ZEROED_COUNTS_D,
	ZEROED_COUNTS_LL,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
};



#[derive(Clone, Copy)]
/// # Randomness.
///
/// This struct is only used to cheaply randomize stat frequencies.
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
	ll_counts: [u32; ZOPFLI_NUM_LL],
	d_counts:  [u32; ZOPFLI_NUM_D],

	pub(crate) ll_symbols: [f64; ZOPFLI_NUM_LL],
	pub(crate) d_symbols:  [f64; ZOPFLI_NUM_D],
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
	/// # Add Previous Stats (Weighted).
	///
	/// This is essentially an `AddAssign` for `ll_counts` and `d_counts`. Each
	/// previous value is halved and added to the corresponding current value.
	pub(crate) fn add_last(
		&mut self,
		ll_counts: &[u32; ZOPFLI_NUM_LL],
		d_counts: &[u32; ZOPFLI_NUM_D],
	) {
		for (l, r) in self.ll_counts.iter_mut().zip(ll_counts.iter().copied()) {
			*l += r.wrapping_div(2);
		}
		for (l, r) in self.d_counts.iter_mut().zip(d_counts.iter().copied()) {
			*l += r.wrapping_div(2);
		}

		// Set the end symbol.
		self.ll_counts[256] = 1;
	}

	/// # Clear Frequencies.
	///
	/// Set all `ll_counts` and `d_counts` to zero and return the originals.
	pub(crate) fn clear(&mut self) -> ([u32; ZOPFLI_NUM_LL], [u32; ZOPFLI_NUM_D]) {
		(
			std::mem::replace(&mut self.ll_counts, ZEROED_COUNTS_LL),
			std::mem::replace(&mut self.d_counts, ZEROED_COUNTS_D),
		)
	}

	/// # Calculate/Set Statistics.
	///
	/// This calculates the "entropy" of the `ll_counts` and `d_counts`, storing the
	/// results in the corresponding symbols arrays.
	pub(crate) fn crunch(&mut self) {
		#[allow(clippy::cast_precision_loss)]
		fn calculate_entropy<const S: usize>(count: &[u32; S], bitlengths: &mut [f64; S]) {
			let sum = count.iter().sum::<u32>();

			if sum == 0 {
				let log2sum = (S as f64).log2();
				bitlengths.fill(log2sum);
			}
			else {
				let log2sum = f64::from(sum).log2();

				for (c, b) in count.iter().copied().zip(bitlengths.iter_mut()) {
					if c == 0 { *b = log2sum; }
					else {
						*b = log2sum - f64::from(c).log2();
						if b.is_sign_negative() { *b = 0.0; }
					}
				}
			}
		}

		calculate_entropy(&self.ll_counts, &mut self.ll_symbols);
		calculate_entropy(&self.d_counts, &mut self.d_symbols);
	}

	/// # Load Statistics.
	///
	/// This updates the `ll_counts` and `d_counts` stats using the data from the
	/// `ZopfliLZ77Store` store, then crunches the results.
	pub(crate) fn load_store(&mut self, store: &LZ77Store) {
		for e in &store.entries {
			if e.dist <= 0 {
				self.ll_counts[e.litlen as usize] += 1;
			}
			else {
				self.ll_counts[e.ll_symbol as usize] += 1;
				self.d_counts[e.d_symbol as usize] += 1;
			}
		}

		// Set the end symbol and crunch.
		self.ll_counts[256] = 1;
		self.crunch();
	}

	/// # Randomize Stat Frequencies.
	///
	/// This randomizes the stat frequencies to allow things to maybe turn out
	/// different on subsequent squeeze passes.
	pub(crate) fn randomize(&mut self, state: &mut RanState) {
		fn randomize_freqs<const S: usize>(freqs: &mut [u32; S], state: &mut RanState) {
			for i in 0..S {
				if (state.randomize() >> 4) % 3 == 0 {
					let index = state.randomize() as usize % S;
					freqs[i] = freqs[index];
				}
			}
		}
		randomize_freqs(&mut self.ll_counts, state);
		randomize_freqs(&mut self.d_counts, state);

		// Set the end symbol.
		self.ll_counts[256] = 1;
	}
}
