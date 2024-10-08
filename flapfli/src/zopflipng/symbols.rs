/*!
# Flapfli: Symbols.

This module contains custom types and lookup tables for length and distance
symbols, bit counts, and bit values, though most of them are actually generated
via `build.rs`.
*/

// This defines the Dsym, LitLen, and Lsym symbol enums, as well as the
// terrible DISTANCE_SYMBOLS and DISTANCE_VALUES lookup tables.
include!(concat!(env!("OUT_DIR"), "/symbols.rs"));

/// # Length Symbols by Litlen.
pub(crate) const LENGTH_SYMBOLS: [Lsym; 259] = [
	Lsym::L000, Lsym::L000, Lsym::L000,
	Lsym::L257, Lsym::L258, Lsym::L259, Lsym::L260, Lsym::L261, Lsym::L262, Lsym::L263, Lsym::L264,
	Lsym::L265, Lsym::L265, Lsym::L266, Lsym::L266, Lsym::L267, Lsym::L267, Lsym::L268, Lsym::L268,
	Lsym::L269, Lsym::L269, Lsym::L269, Lsym::L269, Lsym::L270, Lsym::L270, Lsym::L270, Lsym::L270,
	Lsym::L271, Lsym::L271, Lsym::L271, Lsym::L271, Lsym::L272, Lsym::L272, Lsym::L272, Lsym::L272,
	Lsym::L273, Lsym::L273, Lsym::L273, Lsym::L273, Lsym::L273, Lsym::L273, Lsym::L273, Lsym::L273,
	Lsym::L274, Lsym::L274, Lsym::L274, Lsym::L274, Lsym::L274, Lsym::L274, Lsym::L274, Lsym::L274,
	Lsym::L275, Lsym::L275, Lsym::L275, Lsym::L275, Lsym::L275, Lsym::L275, Lsym::L275, Lsym::L275,
	Lsym::L276, Lsym::L276, Lsym::L276, Lsym::L276, Lsym::L276, Lsym::L276, Lsym::L276, Lsym::L276,
	Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277,
	Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277, Lsym::L277,
	Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278,
	Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278, Lsym::L278,
	Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279,
	Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279, Lsym::L279,
	Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280,
	Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280, Lsym::L280,
	Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281,
	Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281,
	Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281,
	Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281, Lsym::L281,
	Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282,
	Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282,
	Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282,
	Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282, Lsym::L282,
	Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283,
	Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283,
	Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283,
	Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283, Lsym::L283,
	Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284,
	Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284,
	Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284,
	Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L284, Lsym::L285,
];

/// # Length Symbol Bit Values by Litlen.
pub(crate) const LENGTH_SYMBOL_BIT_VALUES: [u8; 259] = [
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 1, 0, 1,
	0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 0, 1, 2, 3, 4, 5, 6, 7,
	0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7, 0, 1, 2, 3, 4, 5, 6, 7,
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7,
	8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 0, 1, 2, 3, 4, 5, 6, 7,
	8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
	24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
	16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 0, 1, 2, 3, 4, 5, 6, 7,
	8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 0,
];

impl DeflateSym {
	/// # Jumbled Tree Symbols.
	///
	/// This ordering is used when encoding DEFLATE trees.
	pub(crate) const TREE: [Self; 19] = [
		Self::D16, Self::D17, Self::D18, Self::D00, Self::D08,
		Self::D07, Self::D09, Self::D06, Self::D10, Self::D05,
		Self::D11, Self::D04, Self::D12, Self::D03, Self::D13,
		Self::D02, Self::D14, Self::D01, Self::D15,
	];

	/// # Is Zero?
	///
	/// Returns `true` if `self` is zero.
	pub(crate) const fn is_zero(self) -> bool { matches!(self, Self::D00) }

	/// # Non-Zero Iter.
	///
	/// Return an iterator beginning at one.
	pub(crate) const fn nonzero_iter() -> DeflateSymIter { DeflateSymIter(1) }
}

impl LitLen {
	/// # Max Matchable.
	///
	/// This is equivalent to `ZOPFLI_MAX_MATCH`.
	pub(crate) const MAX_MATCH: Self = Self::L258;

	/// # Is Matchable?
	///
	/// Returns `true` if `self` is at least `ZOPFLI_MIN_MATCH`.
	pub(crate) const fn is_matchable(self) -> bool { 2 < (self as u16) }

	/// # Is Max?
	///
	/// Returns `true` if `self` is exactly `Self::MAX_MATCH`.
	pub(crate) const fn is_max(self) -> bool { matches!(self, Self::MAX_MATCH) }

	/// # Is Zero?
	///
	/// Returns `true` if `self` is zero.
	pub(crate) const fn is_zero(self) -> bool { matches!(self, Self::L000) }
}

impl LitLen {
	#[expect(unsafe_code, reason = "For transmute.")]
	/// # From U8.
	///
	/// `LitLen` covers the full `u8` range, so we can safely convert the
	/// former into the latter.
	pub(crate) const fn from_u8(n: u8) -> Self {
		// Safety: litlen covers the full range of `u8`.
		unsafe { std::mem::transmute::<u16, Self>(n as u16) }
	}

	#[expect(unsafe_code, reason = "For transmute.")]
	/// # From U8+3.
	///
	/// This reverses the work done by `LitLen::to_packed_u8`, returning the
	/// `LitLen` equivalent of `n + 3`.
	pub(crate) const fn from_packed_u8(n: u8) -> Self {
		// Safety: litlen covers the full range of `u8::MAX + 3`.
		unsafe { std::mem::transmute::<u16, Self>(n as u16 + 3) }
	}

	#[expect(unsafe_code, reason = "For transmute.")]
	/// # Min w/ U16.
	///
	/// Return the smaller of `self` and `n`.
	pub(crate) const fn min_u16(self, n: u16) -> Self {
		if n < (self as u16) {
			// Safety: since n is smaller than self, we know it fits!
			unsafe { std::mem::transmute::<u16, Self>(n) }
		}
		else { self }
	}

	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	#[expect(unsafe_code, reason = "For transmute.")]
	/// # Min w/ Usize.
	///
	/// Return the smaller of `self` and `n`.
	pub(crate) const fn min_usize(self, n: usize) -> Self {
		if n < (self as usize) {
			// Safety: since n is smaller than self, we know it fits!
			unsafe { std::mem::transmute::<u16, Self>(n as u16) }
		}
		else { self }
	}

	/// # Matchable Iter.
	///
	/// Return an iterator covering `ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH`.
	pub(crate) const fn matchable_iter() -> LitLenIter { LitLenIter(3) }

	/// # Next Iter.
	///
	/// Return an iterator beginning with the next litlen, if any.
	pub(crate) const fn next_iter(after: Self) -> LitLenIter {
		LitLenIter(after as u16 + 1)
	}

	#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
	/// # To Packed U8.
	///
	/// This method packs (a matcheable) `LitLen` into a `u8` by subtracting
	/// three. (This works because `LitLen::MAX_MATCH - 3 == u8::MAX`.)
	///
	/// Values less than three shouldn't ever find their way here, but if they
	/// do zero is returned.
	pub(crate) const fn to_packed_u8(self) -> u8 {
		let n = self as u16;
		if 3 < n { (n - 3) as u8 }
		else { 0 }
	}
}

impl Lsym {
	#[expect(unsafe_code, reason = "For transmute.")]
	/// # From `LitLen`.
	///
	/// The full range of `LitLen` is covered by `Lsym`, so the latter can
	/// always represent the former.
	pub(crate) const fn from_litlen(litlen: LitLen) -> Self {
		// Safety: lsym covers the full range of litlen.
		unsafe { std::mem::transmute::<LitLen, Self>(litlen) }
	}
}

impl SplitLen {
	/// # Is Zero?
	///
	/// Returns `true` if `self` is zero.
	pub(crate) const fn is_zero(self) -> bool { matches!(self, Self::S00) }

	/// # Is Max?
	///
	/// Returns `true` if `self` is the maximum value (`SplitLen::S14`).
	pub(crate) const fn is_max(self) -> bool { matches!(self, Self::S14) }
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	#[expect(clippy::float_cmp, reason = "It is what it is.")]
	fn t_symbol_bits() {
		// The DISTANCE_BITS/_F and LENGTH_SYMBOL_BITS/_F constants should have
		// equivalent values.
		for (f, i) in DISTANCE_BITS_F.iter().copied().zip(DISTANCE_BITS) {
			assert_eq!(f, f64::from(i));
		}
		for (f, i) in LENGTH_SYMBOL_BITS_F.iter().copied().zip(LENGTH_SYMBOL_BITS) {
			assert_eq!(f, f64::from(i));
		}
	}

	#[test]
	/// # Deflate Symbol Size and Alignment.
	fn t_deflate_size_align() {
		use super::super::{ArrayD, ArrayLL};

		assert_eq!(
			size_of::<ArrayLL<u8>>(),
			size_of::<ArrayLL<DeflateSym>>(),
		);
		assert_eq!(
			align_of::<ArrayLL<u8>>(),
			align_of::<ArrayLL<DeflateSym>>(),
		);

		assert_eq!(
			size_of::<ArrayD<u8>>(),
			size_of::<ArrayD<DeflateSym>>(),
		);
		assert_eq!(
			align_of::<ArrayD<u8>>(),
			align_of::<ArrayD<DeflateSym>>(),
		);
	}

	// Note: the original `symbols.h` distance-related methods come in two
	// flavors: one leveraging compiler math built-ins and one loaded with
	// manual range branching. These tests compare both approaches against the
	// values in our pre-calculated tables to be doubly-sure we haven't made
	// any silly mistakes. ;)

	#[test]
	/// # Test Distance Symbols (Shortcut).
	fn t_dsym_fast() {
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.

			let sym2 =
				if i < 5 { i.saturating_sub(1) }
				else {
					let d_log = (i - 1).ilog2();
					let r = ((u32::from(i) - 1) >> (d_log - 1)) & 1;
					u16::try_from(d_log * 2 + r).expect("distance bits overflowed")
				};

			assert_eq!(
				sym1 as u16,
				sym2,
				"Wrong distance symbol at {i}; expected {sym2}, found {}", sym1 as u16
			);
		}
	}

	#[test]
	#[expect(clippy::cognitive_complexity, reason = "It is what it is.")]
	/// # Test Distance Symbols (Fallback).
	fn t_dsym_slow() {
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.

			let sym2 =
			if i < 193 {
				// 0..13
				if i < 13 {
					if i < 5 { i.saturating_sub(1) }
					else if i < 7 { 4 }
					else if i < 9 { 5 }
					else { 6 }
				}
				// 13..193
				else if i < 17 { 7 }
				else if i < 25 { 8 }
				else if i < 33 { 9 }
				else if i < 49 { 10 }
				else if i < 65 { 11 }
				else if i < 97 { 12 }
				else if i < 129 { 13 }
				else { 14 }
			}
			else {
				// 193..2049
				if i < 2049 {
					if i < 257 { 15 }
					else if i < 385 { 16 }
					else if i < 513 { 17 }
					else if i < 769 { 18 }
					else if i < 1025 { 19 }
					else if i < 1537 { 20 }
					else { 21 }
				}
				// 2049..32768
				else if i < 3073 { 22 }
				else if i < 4097 { 23 }
				else if i < 6145 { 24 }
				else if i < 8193 { 25 }
				else if i < 12289 { 26 }
				else if i < 16385 { 27 }
				else if i < 24577 { 28 }
				else { 29 }
			};

			assert_eq!(
				sym1 as u16,
				sym2,
				"Wrong distance symbol at {i}; expected {sym2}, found {}", sym1 as u16
			);
		}
	}

	#[test]
	/// # Distance Bits (Shortcut).
	fn t_distance_bits_fast() {
		// The last two positions are unused.
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.
			let bits1 = DISTANCE_BITS[sym1 as usize];

			let bits2 =
				if i < 5 { 0 }
				else {
					u8::try_from((i - 1).ilog2() - 1).expect("distance bits overflowed")
				};

			assert_eq!(
				bits1,
				bits2,
				"Wrong distance bits at {i}; expected {bits2}, found {bits1}"
			);
		}
	}

	#[test]
	/// # Distance Bits (Fallback).
	fn t_distance_bits_slow() {
		// The last two positions are unused.
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.
			let bits1 = DISTANCE_BITS[sym1 as usize];

			let bits2 =
				if i < 5 { 0 }
				else if i < 9 { 1 }
				else if i < 17 { 2 }
				else if i < 33 { 3 }
				else if i < 65 { 4 }
				else if i < 129 { 5 }
				else if i < 257 { 6 }
				else if i < 513 { 7 }
				else if i < 1025 { 8 }
				else if i < 2049 { 9 }
				else if i < 4097 { 10 }
				else if i < 8193 { 11 }
				else if i < 16385 { 12 }
				else { 13 };

			assert_eq!(
				bits1,
				bits2,
				"Wrong distance bits at {i}; expected {bits2}, found {bits1}"
			);
		}
	}

	#[test]
	/// # Distance Bit Values (Shortcut).
	fn t_distance_values_fast() {
		for (i, val1) in DISTANCE_VALUES.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.
			let val2 =
				if i < 5 { 0 }
				else {
					let log2 = (i - 1).ilog2();
					u16::try_from((u32::from(i) - (1 + (1 << log2))) & ((1 << (log2 - 1)) - 1))
						.expect("distance values overflowed")
				};

			assert_eq!(
				val1,
				val2,
				"Wrong distance value at {i}; expected {val2}, found {val1}"
			);
		}
	}

	#[test]
	/// # Distance Bit Values (Fallback).
	fn t_distance_values_slow() {
		for (i, val1) in DISTANCE_VALUES.iter().copied().enumerate() {
			let i = u16::try_from(i).unwrap(); // This can't fail.
			let val2 =
				if i < 5 { 0 }
				else if i < 9 { (i - 5) & 1 }
				else if i < 17 { (i - 9) & 3 }
				else if i < 33 { (i - 17) & 7 }
				else if i < 65 { (i - 33) & 15 }
				else if i < 129 { (i - 65) & 31 }
				else if i < 257 { (i - 129) & 63 }
				else if i < 513 { (i - 257) & 127 }
				else if i < 1025 { (i - 513) & 255 }
				else if i < 2049 { (i - 1025) & 511 }
				else if i < 4097 { (i - 2049) & 1023 }
				else if i < 8193 { (i - 4097) & 2047 }
				else if i < 16385 { (i - 8193) & 4095 }
				else { (i - 16385) & 8191 };

			assert_eq!(
				val1,
				val2,
				"Wrong distance value at {i}; expected {val2}, found {val1}"
			);
		}
	}
}
