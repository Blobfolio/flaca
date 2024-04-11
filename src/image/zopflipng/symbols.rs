/*!
# Flaca: Zopflipng Symbols
*/

// This defines the Dsym, LitLen, and Lsym symbol enums, as well as the
// terrible DISTANCE_SYMBOLS and DISTANCE_VALUES lookup tables.
include!(concat!(env!("OUT_DIR"), "/symbols.rs"));

/// # Distance Extra Bits (by Symbol).
///
/// Note only the first `30` values have meaning, but the compiler doesn't
/// understand distances are only using 15 bits. Padding the table to `32`
/// helps eliminate superfluous bounds checks.
pub(crate) const DISTANCE_BITS: [u8; 32] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
	7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

/// # Length Symbols, Extra Bits, and Bit Values.
///
/// This contains all symbols, bits, and bit values indexed by their
/// litlen.
///
/// This is thankfully much shorter than the distance tables!
pub(crate) const LENGTH_SYMBOLS_BITS_VALUES: [(Lsym, u8, u8); 259] = [
	(Lsym::L000, 0, 0), (Lsym::L000, 0, 0), (Lsym::L000, 0, 0),
	(Lsym::L257, 0, 0), (Lsym::L258, 0, 0), (Lsym::L259, 0, 0), (Lsym::L260, 0, 0), (Lsym::L261, 0, 0), (Lsym::L262, 0, 0), (Lsym::L263, 0, 0), (Lsym::L264, 0, 0),
	(Lsym::L265, 1, 0), (Lsym::L265, 1, 1), (Lsym::L266, 1, 0), (Lsym::L266, 1, 1), (Lsym::L267, 1, 0), (Lsym::L267, 1, 1), (Lsym::L268, 1, 0), (Lsym::L268, 1, 1),
	(Lsym::L269, 2, 0), (Lsym::L269, 2, 1), (Lsym::L269, 2, 2), (Lsym::L269, 2, 3), (Lsym::L270, 2, 0), (Lsym::L270, 2, 1), (Lsym::L270, 2, 2), (Lsym::L270, 2, 3),
	(Lsym::L271, 2, 0), (Lsym::L271, 2, 1), (Lsym::L271, 2, 2), (Lsym::L271, 2, 3), (Lsym::L272, 2, 0), (Lsym::L272, 2, 1), (Lsym::L272, 2, 2), (Lsym::L272, 2, 3),
	(Lsym::L273, 3, 0), (Lsym::L273, 3, 1), (Lsym::L273, 3, 2), (Lsym::L273, 3, 3), (Lsym::L273, 3, 4), (Lsym::L273, 3, 5), (Lsym::L273, 3, 6), (Lsym::L273, 3, 7),
	(Lsym::L274, 3, 0), (Lsym::L274, 3, 1), (Lsym::L274, 3, 2), (Lsym::L274, 3, 3), (Lsym::L274, 3, 4), (Lsym::L274, 3, 5), (Lsym::L274, 3, 6), (Lsym::L274, 3, 7),
	(Lsym::L275, 3, 0), (Lsym::L275, 3, 1), (Lsym::L275, 3, 2), (Lsym::L275, 3, 3), (Lsym::L275, 3, 4), (Lsym::L275, 3, 5), (Lsym::L275, 3, 6), (Lsym::L275, 3, 7),
	(Lsym::L276, 3, 0), (Lsym::L276, 3, 1), (Lsym::L276, 3, 2), (Lsym::L276, 3, 3), (Lsym::L276, 3, 4), (Lsym::L276, 3, 5), (Lsym::L276, 3, 6), (Lsym::L276, 3, 7),
	(Lsym::L277, 4, 0), (Lsym::L277, 4, 1), (Lsym::L277, 4, 2), (Lsym::L277, 4, 3), (Lsym::L277, 4, 4), (Lsym::L277, 4, 5), (Lsym::L277, 4, 6), (Lsym::L277, 4, 7),
	(Lsym::L277, 4, 8), (Lsym::L277, 4, 9), (Lsym::L277, 4, 10), (Lsym::L277, 4, 11), (Lsym::L277, 4, 12), (Lsym::L277, 4, 13), (Lsym::L277, 4, 14), (Lsym::L277, 4, 15),
	(Lsym::L278, 4, 0), (Lsym::L278, 4, 1), (Lsym::L278, 4, 2), (Lsym::L278, 4, 3), (Lsym::L278, 4, 4), (Lsym::L278, 4, 5), (Lsym::L278, 4, 6), (Lsym::L278, 4, 7),
	(Lsym::L278, 4, 8), (Lsym::L278, 4, 9), (Lsym::L278, 4, 10), (Lsym::L278, 4, 11), (Lsym::L278, 4, 12), (Lsym::L278, 4, 13), (Lsym::L278, 4, 14), (Lsym::L278, 4, 15),
	(Lsym::L279, 4, 0), (Lsym::L279, 4, 1), (Lsym::L279, 4, 2), (Lsym::L279, 4, 3), (Lsym::L279, 4, 4), (Lsym::L279, 4, 5), (Lsym::L279, 4, 6), (Lsym::L279, 4, 7),
	(Lsym::L279, 4, 8), (Lsym::L279, 4, 9), (Lsym::L279, 4, 10), (Lsym::L279, 4, 11), (Lsym::L279, 4, 12), (Lsym::L279, 4, 13), (Lsym::L279, 4, 14), (Lsym::L279, 4, 15),
	(Lsym::L280, 4, 0), (Lsym::L280, 4, 1), (Lsym::L280, 4, 2), (Lsym::L280, 4, 3), (Lsym::L280, 4, 4), (Lsym::L280, 4, 5), (Lsym::L280, 4, 6), (Lsym::L280, 4, 7),
	(Lsym::L280, 4, 8), (Lsym::L280, 4, 9), (Lsym::L280, 4, 10), (Lsym::L280, 4, 11), (Lsym::L280, 4, 12), (Lsym::L280, 4, 13), (Lsym::L280, 4, 14), (Lsym::L280, 4, 15),
	(Lsym::L281, 5, 0), (Lsym::L281, 5, 1), (Lsym::L281, 5, 2), (Lsym::L281, 5, 3), (Lsym::L281, 5, 4), (Lsym::L281, 5, 5), (Lsym::L281, 5, 6), (Lsym::L281, 5, 7),
	(Lsym::L281, 5, 8), (Lsym::L281, 5, 9), (Lsym::L281, 5, 10), (Lsym::L281, 5, 11), (Lsym::L281, 5, 12), (Lsym::L281, 5, 13), (Lsym::L281, 5, 14), (Lsym::L281, 5, 15),
	(Lsym::L281, 5, 16), (Lsym::L281, 5, 17), (Lsym::L281, 5, 18), (Lsym::L281, 5, 19), (Lsym::L281, 5, 20), (Lsym::L281, 5, 21), (Lsym::L281, 5, 22), (Lsym::L281, 5, 23),
	(Lsym::L281, 5, 24), (Lsym::L281, 5, 25), (Lsym::L281, 5, 26), (Lsym::L281, 5, 27), (Lsym::L281, 5, 28), (Lsym::L281, 5, 29), (Lsym::L281, 5, 30), (Lsym::L281, 5, 31),
	(Lsym::L282, 5, 0), (Lsym::L282, 5, 1), (Lsym::L282, 5, 2), (Lsym::L282, 5, 3), (Lsym::L282, 5, 4), (Lsym::L282, 5, 5), (Lsym::L282, 5, 6), (Lsym::L282, 5, 7),
	(Lsym::L282, 5, 8), (Lsym::L282, 5, 9), (Lsym::L282, 5, 10), (Lsym::L282, 5, 11), (Lsym::L282, 5, 12), (Lsym::L282, 5, 13), (Lsym::L282, 5, 14), (Lsym::L282, 5, 15),
	(Lsym::L282, 5, 16), (Lsym::L282, 5, 17), (Lsym::L282, 5, 18), (Lsym::L282, 5, 19), (Lsym::L282, 5, 20), (Lsym::L282, 5, 21), (Lsym::L282, 5, 22), (Lsym::L282, 5, 23),
	(Lsym::L282, 5, 24), (Lsym::L282, 5, 25), (Lsym::L282, 5, 26), (Lsym::L282, 5, 27), (Lsym::L282, 5, 28), (Lsym::L282, 5, 29), (Lsym::L282, 5, 30), (Lsym::L282, 5, 31),
	(Lsym::L283, 5, 0), (Lsym::L283, 5, 1), (Lsym::L283, 5, 2), (Lsym::L283, 5, 3), (Lsym::L283, 5, 4), (Lsym::L283, 5, 5), (Lsym::L283, 5, 6), (Lsym::L283, 5, 7),
	(Lsym::L283, 5, 8), (Lsym::L283, 5, 9), (Lsym::L283, 5, 10), (Lsym::L283, 5, 11), (Lsym::L283, 5, 12), (Lsym::L283, 5, 13), (Lsym::L283, 5, 14), (Lsym::L283, 5, 15),
	(Lsym::L283, 5, 16), (Lsym::L283, 5, 17), (Lsym::L283, 5, 18), (Lsym::L283, 5, 19), (Lsym::L283, 5, 20), (Lsym::L283, 5, 21), (Lsym::L283, 5, 22), (Lsym::L283, 5, 23),
	(Lsym::L283, 5, 24), (Lsym::L283, 5, 25), (Lsym::L283, 5, 26), (Lsym::L283, 5, 27), (Lsym::L283, 5, 28), (Lsym::L283, 5, 29), (Lsym::L283, 5, 30), (Lsym::L283, 5, 31),
	(Lsym::L284, 5, 0), (Lsym::L284, 5, 1), (Lsym::L284, 5, 2), (Lsym::L284, 5, 3), (Lsym::L284, 5, 4), (Lsym::L284, 5, 5), (Lsym::L284, 5, 6), (Lsym::L284, 5, 7),
	(Lsym::L284, 5, 8), (Lsym::L284, 5, 9), (Lsym::L284, 5, 10), (Lsym::L284, 5, 11), (Lsym::L284, 5, 12), (Lsym::L284, 5, 13), (Lsym::L284, 5, 14), (Lsym::L284, 5, 15),
	(Lsym::L284, 5, 16), (Lsym::L284, 5, 17), (Lsym::L284, 5, 18), (Lsym::L284, 5, 19), (Lsym::L284, 5, 20), (Lsym::L284, 5, 21), (Lsym::L284, 5, 22), (Lsym::L284, 5, 23),
	(Lsym::L284, 5, 24), (Lsym::L284, 5, 25), (Lsym::L284, 5, 26), (Lsym::L284, 5, 27), (Lsym::L284, 5, 28), (Lsym::L284, 5, 29), (Lsym::L284, 5, 30), (Lsym::L285, 0, 0),
];



#[cfg(test)]
mod tests {
	use super::*;

	// Note: the original `symbols.h` distance-related methods come in two
	// flavors: one leveraging compiler math built-ins and one loaded with
	// manual range branching. These tests compare both approaches against the
	// values in our pre-calculated tables to be doubly-sure we haven't made
	// any silly mistakes. ;)

	#[test]
	/// # Test Distance Symbols (Shortcut).
	fn t_dsym_fast() {
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = i as u16;

			let sym2 =
				if i < 5 { i.saturating_sub(1) }
				else {
					let d_log = (i - 1).ilog2();
					let r = ((i as u32 - 1) >> (d_log - 1)) & 1;
					(d_log * 2 + r) as u16
				};

			assert_eq!(
				sym1 as u16,
				sym2,
				"Wrong distance symbol at {i}; expected {sym2}, found {}", sym1 as u16
			);
		}
	}

	#[test]
	/// # Test Distance Symbols (Fallback).
	fn t_dsym_slow() {
		for (i, sym1) in DISTANCE_SYMBOLS.iter().copied().enumerate() {
			let i = i as u16;

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
				else {
					if i < 17 { 7 }
					else if i < 25 { 8 }
					else if i < 33 { 9 }
					else if i < 49 { 10 }
					else if i < 65 { 11 }
					else if i < 97 { 12 }
					else if i < 129 { 13 }
					else { 14 }
				}
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
				else {
					if i < 3073 { 22 }
					else if i < 4097 { 23 }
					else if i < 6145 { 24 }
					else if i < 8193 { 25 }
					else if i < 12289 { 26 }
					else if i < 16385 { 27 }
					else if i < 24577 { 28 }
					else { 29 }
				}
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
			let i = i as u16;
			let bits1 = DISTANCE_BITS[sym1 as usize];

			let bits2 =
				if i < 5 { 0 }
				else {
					((i - 1).ilog2() - 1) as u8
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
			let i = i as u16;
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
			let i = i as u16;
			let val2 =
				if i < 5 { 0 }
				else {
					let log2 = (i - 1).ilog2();
					((i as u32 - (1 + (1 << log2))) & ((1 << (log2 - 1)) - 1)) as u16
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
			let i = i as u16;
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
