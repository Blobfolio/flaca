/*!
# Flaca: Zopflipng Squeeze Stats.

This replaces the `SymbolStats` and `RanState` functionality originally found
in `squeeze.c`.
*/

use super::{
	distance_symbol,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliLZ77Store,
};



/// # Length Symbols.
const LENGTH_SYMBOLS: [u16; 259] = [
	0, 0, 0,
	257, 258, 259, 260, 261, 262, 263, 264,
	265, 265, 266, 266, 267, 267, 268, 268,
	269, 269, 269, 269, 270, 270, 270, 270,
	271, 271, 271, 271, 272, 272, 272, 272,
	273, 273, 273, 273, 273, 273, 273, 273,
	274, 274, 274, 274, 274, 274, 274, 274,
	275, 275, 275, 275, 275, 275, 275, 275,
	276, 276, 276, 276, 276, 276, 276, 276,
	277, 277, 277, 277, 277, 277, 277, 277,
	277, 277, 277, 277, 277, 277, 277, 277,
	278, 278, 278, 278, 278, 278, 278, 278,
	278, 278, 278, 278, 278, 278, 278, 278,
	279, 279, 279, 279, 279, 279, 279, 279,
	279, 279, 279, 279, 279, 279, 279, 279,
	280, 280, 280, 280, 280, 280, 280, 280,
	280, 280, 280, 280, 280, 280, 280, 280,
	281, 281, 281, 281, 281, 281, 281, 281,
	281, 281, 281, 281, 281, 281, 281, 281,
	281, 281, 281, 281, 281, 281, 281, 281,
	281, 281, 281, 281, 281, 281, 281, 281,
	282, 282, 282, 282, 282, 282, 282, 282,
	282, 282, 282, 282, 282, 282, 282, 282,
	282, 282, 282, 282, 282, 282, 282, 282,
	282, 282, 282, 282, 282, 282, 282, 282,
	283, 283, 283, 283, 283, 283, 283, 283,
	283, 283, 283, 283, 283, 283, 283, 283,
	283, 283, 283, 283, 283, 283, 283, 283,
	283, 283, 283, 283, 283, 283, 283, 283,
	284, 284, 284, 284, 284, 284, 284, 284,
	284, 284, 284, 284, 284, 284, 284, 284,
	284, 284, 284, 284, 284, 284, 284, 284,
	284, 284, 284, 284, 284, 284, 284, 285,
];



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
	/// This uses the 32-bit "Multiply-With-Carry" generator (G. Marsaglia).
	fn randomize(&mut self) -> u32 {
		self.m_z = 36_969 * (self.m_z & 65_535) + (self.m_z >> 16);
		self.m_w = 18_000 * (self.m_w & 65_535) + (self.m_w >> 16);
		(self.m_z << 16).wrapping_add(self.m_w) // 32-bit result.
	}
}



#[derive(Clone, Copy)]
/// # Symbol Stats.
///
/// This holds the length and distance symbols and costs for a given block,
/// data that can be used to improve compression on subsequent passes.
pub(crate) struct SymbolStats {
	litlens: [usize; ZOPFLI_NUM_LL],
	dists:   [usize; ZOPFLI_NUM_D],

	pub(crate) ll_symbols: [f64; ZOPFLI_NUM_LL],
	pub(crate) d_symbols:  [f64; ZOPFLI_NUM_D],
}

impl SymbolStats {
	/// # New Instance.
	pub(crate) const fn new() -> Self {
		Self {
			litlens:    [0; ZOPFLI_NUM_LL],
			dists:      [0; ZOPFLI_NUM_D],

			ll_symbols: [0.0; ZOPFLI_NUM_LL],
			d_symbols:  [0.0; ZOPFLI_NUM_D],
		}
	}
}

impl SymbolStats {
	/// # Add Previous Stats (Weighted).
	///
	/// This is essentially an `AddAssign` for `litlens` and `dists`. Each
	/// previous value is halved and added to the corresponding current value.
	pub(crate) fn add_last(
		&mut self,
		litlens: &[usize; ZOPFLI_NUM_LL],
		dists: &[usize; ZOPFLI_NUM_D],
	) {
		for (l, r) in self.litlens.iter_mut().zip(litlens.iter().copied()) {
			*l += r.wrapping_div(2);
		}
		for (l, r) in self.dists.iter_mut().zip(dists.iter().copied()) {
			*l += r.wrapping_div(2);
		}

		// Set the end symbol.
		self.litlens[256] = 1;
	}

	/// # Clear Frequencies.
	///
	/// Set all `litlens` and `dists` to zero and return the originals.
	pub(crate) fn clear(&mut self) -> ([usize; ZOPFLI_NUM_LL], [usize; ZOPFLI_NUM_D]) {
		let mut new_litlens = [0; ZOPFLI_NUM_LL];
		let mut new_dists = [0; ZOPFLI_NUM_D];
		std::mem::swap(&mut self.litlens, &mut new_litlens);
		std::mem::swap(&mut self.dists, &mut new_dists);
		(new_litlens, new_dists)
	}

	/// # Calculate/Set Statistics.
	///
	/// This calculates the "entropy" of the `litlens` and `dists`, storing the
	/// results in the corresponding symbols arrays.
	pub(crate) fn crunch(&mut self) {
		#[allow(clippy::cast_precision_loss)]
		fn calculate_entropy<const S: usize>(count: &[usize; S], bitlengths: &mut [f64; S]) {
			let sum = count.iter().sum::<usize>();

			if sum == 0 {
				let log2sum = (S as f64).log2();
				bitlengths.fill(log2sum);
			}
			else {
				let log2sum = (sum as f64).log2();

				for (&c, b) in count.iter().zip(bitlengths.iter_mut()) {
					if c == 0 { *b = log2sum; }
					else {
						let mut v = log2sum - (c as f64).log2();
						if v.is_sign_negative() { v = 0.0; }
						*b = v;
					}
				}
			}
		}

		calculate_entropy::<ZOPFLI_NUM_LL>(&self.litlens, &mut self.ll_symbols);
		calculate_entropy::<ZOPFLI_NUM_D>(&self.dists, &mut self.d_symbols);
	}

	#[allow(unsafe_code, clippy::similar_names)]
	/// # Load Statistics.
	///
	/// This updates the `litlens` and `dists` stats using the data from the
	/// `ZopfliLZ77Store` store, then crunches the results.
	pub(crate) fn load_store(&mut self, store: &ZopfliLZ77Store) {
		for i in 0..store.size {
			// We'll need both of these values regardless of what they
			// represent.
			let litlen = usize::from(unsafe { *store.litlens.add(i) });
			let dist = u32::from(unsafe { *store.dists.add(i) });

			if 0 == dist {
				// Safety: `ZopfliStoreLitLenDist` asserts lengths are < 259
				// before storing them.
				unsafe { *self.litlens.get_unchecked_mut(litlen) += 1; }
			}
			else {
				// Safety: all LENGTH_SYMBOLS are in range.
				let lsym = usize::from(unsafe { *LENGTH_SYMBOLS.get_unchecked(litlen) });
				unsafe { *self.litlens.get_unchecked_mut(lsym) += 1; }

				let dsym = distance_symbol(dist);
				self.dists[dsym] += 1;
			}
		}

		// Set the end symbol and crunch.
		self.litlens[256] = 1;
		self.crunch();
	}

	/// # Randomize Stat Frequencies.
	///
	/// This randomizes the stat frequencies to allow things to maybe turn out
	/// different on subsequent squeeze passes.
	pub(crate) fn randomize(&mut self, state: &mut RanState) {
		fn randomize_freqs<const S: usize>(freqs: &mut [usize; S], state: &mut RanState) {
			for i in 0..S {
				if (state.randomize() >> 4) % 3 == 0 {
					let index = state.randomize() as usize % S;
					freqs[i] = freqs[index];
				}
			}
		}
		randomize_freqs::<ZOPFLI_NUM_LL>(&mut self.litlens, state);
		randomize_freqs::<ZOPFLI_NUM_D>(&mut self.dists, state);

		// Set the end symbol.
		self.litlens[256] = 1;
	}
}
