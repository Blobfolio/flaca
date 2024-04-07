/*!
# Flaca: Zopflipng Blocks.
*/

use dactyl::NoHash;
use std::collections::HashSet;
use super::{
	CACHE,
	distance_symbol,
	FIXED_TREE_D,
	FIXED_TREE_LL,
	HASH,
	lz77_optimal,
	LZ77Store,
	SQUEEZE,
	zopfli_length_limited_code_lengths,
	zopfli_lengths_to_symbols,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliAddBit,
	ZopfliAddBits,
	ZopfliAddHuffmanBits,
	ZopfliAddNonCompressedBlock,
	ZopfliAppendLZ77Store,
	ZopfliEncodeTree,
	ZopfliLZ77Store,
};



/// # Distance Extra Bits.
///
/// Note the last two are unused, but included to help eliminate bounds
/// checks.
const DISTANCE_EXTRA_BITS: [u8; 32] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
	7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

/// # Distance Extra Bits Value Masks.
const DISTANCE_EXTRA_BITS_MASK: [(u32, u32); 16] = [
	(0, 0), (0, 0), (5, 1), (9, 3), (17, 7), (33, 15), (65, 31), (129, 63),
	(257, 127), (513, 255), (1025, 511), (2049, 1023), (4097, 2047),
	(8193, 4095), (16_385, 8191), (32_769, 16_383),
];

/// # Length Extra Bits.
const LENGTH_EXTRA_BITS: [u8; 29] = [
	0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
	3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// # Length Symbols, Extra Bits, and Bit Values.
const LENGTH_SYMBOLS_BITS_VALUES: [(usize, u8, u8); 259] = [
	(0, 0, 0), (0, 0, 0), (0, 0, 0),
	(257, 0, 0), (258, 0, 0), (259, 0, 0), (260, 0, 0), (261, 0, 0), (262, 0, 0), (263, 0, 0), (264, 0, 0),
	(265, 1, 0), (265, 1, 1), (266, 1, 0), (266, 1, 1), (267, 1, 0), (267, 1, 1), (268, 1, 0), (268, 1, 1),
	(269, 2, 0), (269, 2, 1), (269, 2, 2), (269, 2, 3), (270, 2, 0), (270, 2, 1), (270, 2, 2), (270, 2, 3),
	(271, 2, 0), (271, 2, 1), (271, 2, 2), (271, 2, 3), (272, 2, 0), (272, 2, 1), (272, 2, 2), (272, 2, 3),
	(273, 3, 0), (273, 3, 1), (273, 3, 2), (273, 3, 3), (273, 3, 4), (273, 3, 5), (273, 3, 6), (273, 3, 7),
	(274, 3, 0), (274, 3, 1), (274, 3, 2), (274, 3, 3), (274, 3, 4), (274, 3, 5), (274, 3, 6), (274, 3, 7),
	(275, 3, 0), (275, 3, 1), (275, 3, 2), (275, 3, 3), (275, 3, 4), (275, 3, 5), (275, 3, 6), (275, 3, 7),
	(276, 3, 0), (276, 3, 1), (276, 3, 2), (276, 3, 3), (276, 3, 4), (276, 3, 5), (276, 3, 6), (276, 3, 7),
	(277, 4, 0), (277, 4, 1), (277, 4, 2), (277, 4, 3), (277, 4, 4), (277, 4, 5), (277, 4, 6), (277, 4, 7),
	(277, 4, 8), (277, 4, 9), (277, 4, 10), (277, 4, 11), (277, 4, 12), (277, 4, 13), (277, 4, 14), (277, 4, 15),
	(278, 4, 0), (278, 4, 1), (278, 4, 2), (278, 4, 3), (278, 4, 4), (278, 4, 5), (278, 4, 6), (278, 4, 7),
	(278, 4, 8), (278, 4, 9), (278, 4, 10), (278, 4, 11), (278, 4, 12), (278, 4, 13), (278, 4, 14), (278, 4, 15),
	(279, 4, 0), (279, 4, 1), (279, 4, 2), (279, 4, 3), (279, 4, 4), (279, 4, 5), (279, 4, 6), (279, 4, 7),
	(279, 4, 8), (279, 4, 9), (279, 4, 10), (279, 4, 11), (279, 4, 12), (279, 4, 13), (279, 4, 14), (279, 4, 15),
	(280, 4, 0), (280, 4, 1), (280, 4, 2), (280, 4, 3), (280, 4, 4), (280, 4, 5), (280, 4, 6), (280, 4, 7),
	(280, 4, 8), (280, 4, 9), (280, 4, 10), (280, 4, 11), (280, 4, 12), (280, 4, 13), (280, 4, 14), (280, 4, 15),
	(281, 5, 0), (281, 5, 1), (281, 5, 2), (281, 5, 3), (281, 5, 4), (281, 5, 5), (281, 5, 6), (281, 5, 7),
	(281, 5, 8), (281, 5, 9), (281, 5, 10), (281, 5, 11), (281, 5, 12), (281, 5, 13), (281, 5, 14), (281, 5, 15),
	(281, 5, 16), (281, 5, 17), (281, 5, 18), (281, 5, 19), (281, 5, 20), (281, 5, 21), (281, 5, 22), (281, 5, 23),
	(281, 5, 24), (281, 5, 25), (281, 5, 26), (281, 5, 27), (281, 5, 28), (281, 5, 29), (281, 5, 30), (281, 5, 31),
	(282, 5, 0), (282, 5, 1), (282, 5, 2), (282, 5, 3), (282, 5, 4), (282, 5, 5), (282, 5, 6), (282, 5, 7),
	(282, 5, 8), (282, 5, 9), (282, 5, 10), (282, 5, 11), (282, 5, 12), (282, 5, 13), (282, 5, 14), (282, 5, 15),
	(282, 5, 16), (282, 5, 17), (282, 5, 18), (282, 5, 19), (282, 5, 20), (282, 5, 21), (282, 5, 22), (282, 5, 23),
	(282, 5, 24), (282, 5, 25), (282, 5, 26), (282, 5, 27), (282, 5, 28), (282, 5, 29), (282, 5, 30), (282, 5, 31),
	(283, 5, 0), (283, 5, 1), (283, 5, 2), (283, 5, 3), (283, 5, 4), (283, 5, 5), (283, 5, 6), (283, 5, 7),
	(283, 5, 8), (283, 5, 9), (283, 5, 10), (283, 5, 11), (283, 5, 12), (283, 5, 13), (283, 5, 14), (283, 5, 15),
	(283, 5, 16), (283, 5, 17), (283, 5, 18), (283, 5, 19), (283, 5, 20), (283, 5, 21), (283, 5, 22), (283, 5, 23),
	(283, 5, 24), (283, 5, 25), (283, 5, 26), (283, 5, 27), (283, 5, 28), (283, 5, 29), (283, 5, 30), (283, 5, 31),
	(284, 5, 0), (284, 5, 1), (284, 5, 2), (284, 5, 3), (284, 5, 4), (284, 5, 5), (284, 5, 6), (284, 5, 7),
	(284, 5, 8), (284, 5, 9), (284, 5, 10), (284, 5, 11), (284, 5, 12), (284, 5, 13), (284, 5, 14), (284, 5, 15),
	(284, 5, 16), (284, 5, 17), (284, 5, 18), (284, 5, 19), (284, 5, 20), (284, 5, 21), (284, 5, 22), (284, 5, 23),
	(284, 5, 24), (284, 5, 25), (284, 5, 26), (284, 5, 27), (284, 5, 28), (284, 5, 29), (284, 5, 30), (285, 0, 0),
];

/// # Minimum Split Distance.
const MINIMUM_SPLIT_DISTANCE: usize = 10;

/// # Max Split Points.
const MAX_SPLIT_POINTS: usize = 14;



#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
/// # Block Type.
pub(super) enum BlockType {
	Uncompressed = 0_u8,
	Fixed = 1_u8,
	Dynamic = 2_u8,
}



/// # Split Point Scratch.
///
/// This holds three sets of block split points for use during the deflate
/// passes. Each set can hold up to 14 points (one less than
/// `BLOCKSPLITTING_MAX`).
///
/// A single instance of this struct is (re)used for all deflate passes on a
/// given image to reduce allocation overhead.
pub(crate) struct SplitPoints {
	slice1: [usize; MAX_SPLIT_POINTS],
	slice2: [usize; MAX_SPLIT_POINTS],
	done: HashSet<usize, NoHash>,
}

impl SplitPoints {
	/// # New Instance.
	pub(crate) fn new() -> Self {
		Self {
			slice1: [0; MAX_SPLIT_POINTS],
			slice2: [0; MAX_SPLIT_POINTS],
			done: HashSet::with_hasher(NoHash::default()),
		}
	}
}

impl SplitPoints {
	#[allow(unsafe_code)]
	/// # Uncompressed Split Pass.
	///
	/// This sets the uncompressed split points, by way of first setting the
	/// LZ77 split points.
	///
	/// In terms of order-of-operations, this must be called _before_ the
	/// second-stage LZ77 pass as it would otherwise blow away that data.
	fn split_raw(&mut self, arr: *const u8, instart: usize, inend: usize) -> usize {
		// Populate an LZ77 store from a greedy pass. This results in better
		// block choices than a full optimal pass.
		let mut store = LZ77Store::new(arr);
		HASH.with_borrow_mut(|h| h.greedy(
			arr,
			instart,
			inend,
			&mut store,
			None,
		));

		// Do an LZ77 pass.
		let len = self.split_lz77(&store);

		// Find the corresponding uncompressed positions.
		if 0 < len {
			// All positions should be within range of the store…
			debug_assert!(self.slice2[len - 1] < store.size);

			let mut pos = instart;
			let mut i = 0;
			for (raw, &lz77) in self.slice1.iter_mut().zip(self.slice2.iter()).take(len) {
				while i <= lz77 {
					let length = unsafe {
						if *store.dists.add(i) == 0 { 1 }
						else { *store.litlens.add(i) }
					};

					if i == lz77 { *raw = pos; }

					pos += usize::from(length);
					i += 1;
				}
			}
		}

		len
	}

	/// # LZ77 Split Pass.
	///
	/// This sets the LZ77 split points according to convoluted cost
	/// evaluations.
	fn split_lz77(&mut self, store: &ZopfliLZ77Store) -> usize {
		// This won't work on tiny files.
		if store.size < MINIMUM_SPLIT_DISTANCE { return 0; }

		// Get started!
		self.done.clear();
		let mut lstart = 0;
		let mut lend = store.size;
		let mut last = 0;
		let mut len = 0;
		loop {
			let (llpos, llcost) = find_minimum_cost(store, lstart + 1, lend);
			assert!(lstart < llpos && llpos < lend);

			// Ignore points we've already covered.
			if llpos == lstart + 1 || estimate_cost(store, lstart, lend) < llcost {
				self.done.insert(lstart);
			}
			else {
				// Mark it as a split point and add it sorted.
				self.slice2[len] = llpos;
				len += 1;

				// Keep the list sorted.
				if last > llpos { self.slice2[..len].sort_unstable(); }
				else { last = llpos; }

				// Stop if we've split the maximum number of times.
				if len == MAX_SPLIT_POINTS { break; }
			}

			// Look for a split and adjust the start/end accordingly. If we don't
			// find one or the remaining distance is too small to continue, we're
			// done!
			if ! find_largest_splittable_block(
				store.size,
				&self.done,
				&self.slice2[..len],
				&mut lstart,
				&mut lend,
			) { break; }
		}

		len
	}

	#[allow(unsafe_code)]
	/// # Split Best.
	fn split(
		&mut self,
		numiterations: i32,
		arr: *const u8,
		instart: usize,
		inend: usize,
		store: &mut ZopfliLZ77Store,
	) -> &[usize] {
		// Start by splitting uncompressed.
		let limit = self.split_raw(arr, instart, inend).min(MAX_SPLIT_POINTS);

		let mut cost1 = 0;
		for i in 0..=limit {
			let start = if i == 0 { instart } else { self.slice1[i - 1] };
			let end = if i < limit { self.slice1[i] } else { inend };

			// Make another store.
			let mut store2 = LZ77Store::new(arr);
			lz77_optimal(arr, start, end, numiterations, &mut store2);
			cost1 += calculate_block_size_auto_type(&store2, 0, store2.size);

			// Append its data to our main store.
			unsafe { ZopfliAppendLZ77Store(&*store2, &mut *store); }

			// Save the chunk size to our best.
			if i < limit { self.slice2[i] = store.size; }
		}

		// Try a second pass, recalculating the LZ77 splits with the updated
		// store details.
		if 1 < limit {
			// Move slice2 over to slice1 so we can repopulate slice2.
			self.slice1.copy_from_slice(self.slice2.as_slice());

			let limit2 = self.split_lz77(store).min(MAX_SPLIT_POINTS);
			let mut cost2 = 0;
			for i in 0..=limit2 {
				let start = if i == 0 { 0 } else { self.slice2[i - 1] };
				let end = if i < limit2 { self.slice2[i] } else { store.size };
				cost2 += calculate_block_size_auto_type(store, start, end);
			}

			// It's better!
			if cost2 < cost1 { &self.slice2[..limit2] }
			else { &self.slice1[..limit] }
		}
		else { &self.slice2[..limit] }
	}
}



/// # Calculate Block Size (in Bits).
pub(super) fn calculate_block_size(
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	btype: BlockType,
) -> usize {
	match btype {
		BlockType::Uncompressed => {
			let length = get_lz77_byte_range(store, lstart, lend);
			let blocks = length.div_ceil(65_535);

			// Blocks larger than u16::MAX need to be split.
			blocks * 40 + length * 8
		},
		BlockType::Fixed =>
			calculate_block_symbol_size(
				&FIXED_TREE_LL,
				&FIXED_TREE_D,
				store,
				lstart,
				lend,
			) + 3,
		BlockType::Dynamic => {
			let mut ll_lengths = [0_u32; ZOPFLI_NUM_LL];
			let mut d_lengths = [0_u32; ZOPFLI_NUM_D];
			get_dynamic_lengths(
				store,
				lstart,
				lend,
				&mut ll_lengths,
				&mut d_lengths,
			)
		},
	}
}

#[allow(unsafe_code, clippy::too_many_arguments)]
/// # Deflate Part.
pub(crate) fn deflate_part(
	splits: &mut SplitPoints,
	numiterations: i32,
	last_block: bool,
	arr: *const u8,
	instart: usize,
	inend: usize,
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) {
	// Find the split points.
	let mut store = LZ77Store::new(arr);
	let best = splits.split(
		numiterations,
		arr,
		instart,
		inend,
		&mut store,
	);

	// Write the data!
	for i in 0..=best.len() {
		let start = if i == 0 { 0 } else { best[i - 1] };
		let end = if i < best.len() { best[i] } else { store.size };
		add_lz77_block_auto_type(
			i == best.len() && last_block,
			&store,
			start,
			end,
			0,
			bp,
			out,
			outsize,
		);
	}
}



/// # RLE-Optimized Stretches.
///
/// This iterator yields a boolean value for each entry of the source slice,
/// `true` for distance codes in a sequence of 5+ zeroes or 7+ (identical)
/// non-zeroes, `false` otherwise.
///
/// (Such ranges are already RLE-optimal.)
struct GoodForRle<'a> {
	counts: &'a [usize],
	good: usize,
	bad: usize,
}

impl<'a> GoodForRle<'a> {
	/// # New Instance.
	const fn new(counts: &'a [usize]) -> Self {
		Self { counts, good: 0, bad: 0 }
	}
}

impl<'a> Iterator for GoodForRle<'a> {
	type Item = bool;

	fn next(&mut self) -> Option<Self::Item> {
		// Return good or bad values from the buffer.
		if self.good != 0 {
			self.good -= 1;
			return Some(true);
		}
		if self.bad != 0 {
			self.bad -= 1;
			return Some(false);
		}

		// If the slice is empty, we're done!
		if self.counts.is_empty() { return None; }

		// See how many times the next entry is repeated, if at all, shortening
		// the slice accordingly.
		let scratch = self.counts[0];
		let mut stride = 0;
		while let [count, rest @ ..] = self.counts {
			// Note the reptition and circle back around.
			if *count == scratch {
				stride += 1;
				self.counts = rest;
			}
			// We had an optimal stretch.
			else if stride >= 5 && (scratch == 0 || stride >= 7) {
				self.good = stride - 1;
				return Some(true);
			}
			// We had a non-optimal stretch.
			else {
				self.bad = stride - 1;
				return Some(false);
			}
		}

		// Finish up by qualifying the dangling stride as optimal or not.
		if stride >= 5 && (scratch == 0 || stride >= 7) {
			self.good = stride - 1;
			Some(true)
		}
		else {
			self.bad = stride - 1;
			Some(false)
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for GoodForRle<'a> {
	fn len(&self) -> usize { self.good + self.bad + self.counts.len() }
}



#[allow(unsafe_code, clippy::too_many_arguments)]
/// # Add LZ77 Block.
///
/// Add a deflate block with the given LZ77 data to the output.
fn add_lz77_block(
	btype: BlockType,
	last_block: bool,
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) {
	// Uncompressed blocks are easy!
	if matches!(btype, BlockType::Uncompressed) {
		let length = get_lz77_byte_range(store, lstart, lend);
		let pos =
			if lstart >= lend { 0 }
			else {
				unsafe { *store.pos.add(lstart) }
			};
		unsafe {
			ZopfliAddNonCompressedBlock(
				i32::from(last_block),
				store.data,
				pos,
				pos + length,
				bp,
				out,
				outsize,
			);
		}
		return;
	}

	// Add some bits.
	unsafe {
		ZopfliAddBit(i32::from(last_block), bp, out, outsize);
		ZopfliAddBit((btype as i32) & 1, bp, out, outsize);
		ZopfliAddBit(((btype as i32) & 2) >> 1, bp, out, outsize);
	}

	// Sort out the lengths, fixed or dynamic.
	let (ll_lengths, d_lengths) =
		if matches!(btype, BlockType::Fixed) { (FIXED_TREE_LL, FIXED_TREE_D) }
		else {
			let mut ll_lengths = [0_u32; ZOPFLI_NUM_LL];
			let mut d_lengths = [0_u32; ZOPFLI_NUM_D];
			get_dynamic_lengths(
				store,
				lstart,
				lend,
				&mut ll_lengths,
				&mut d_lengths,
			);
			add_dynamic_tree(ll_lengths.as_ptr(), d_lengths.as_ptr(), bp, out, outsize);
			(ll_lengths, d_lengths)
		};

	// Now sort out the symbols.
	let mut ll_symbols = [0_u32; ZOPFLI_NUM_LL];
	let mut d_symbols = [0_u32; ZOPFLI_NUM_D];
	zopfli_lengths_to_symbols::<16, ZOPFLI_NUM_LL>(&ll_lengths, &mut ll_symbols);
	zopfli_lengths_to_symbols::<16, ZOPFLI_NUM_D>(&d_lengths, &mut d_symbols);

	// Write all the data!
	add_lz77_data(
		store, lstart, lend, expected_data_size,
		&ll_symbols, &ll_lengths, &d_symbols, &d_lengths,
		bp, out, outsize
	);

	// Finish up by writting the end symbol.
	unsafe {
		ZopfliAddHuffmanBits(ll_symbols[256], ll_lengths[256], bp, out, outsize);
	}
}

#[allow(
	unsafe_code,
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::too_many_arguments,
)]
/// # Add LZ77 Block (Automatic Type).
fn add_lz77_block_auto_type(
	last_block: bool,
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) {
	// If the block is empty, we can assume a fixed-tree layout.
	if lstart >= lend {
		unsafe {
			ZopfliAddBits(u32::from(last_block), 1, bp, out, outsize);
			ZopfliAddBits(1, 2, bp, out, outsize);
			ZopfliAddBits(0, 7, bp, out, outsize);
		}
		return;
	}

	// Calculate the three costs.
	let uncompressed_cost = calculate_block_size(store, lstart, lend, BlockType::Uncompressed);
	let fixed_cost = calculate_block_size(store, lstart, lend, BlockType::Fixed);
	let dynamic_cost = calculate_block_size(store, lstart, lend, BlockType::Dynamic);

	// Fixed stores are only useful up to a point; we can skip the overhead
	// if the store is big or the dynamic cost estimate is unimpressive.
	if
		(store.size < 1000 || (fixed_cost as f64) <= (dynamic_cost as f64) * 1.1) &&
		try_lz77_expensive_fixed(
			store, uncompressed_cost, dynamic_cost,
			lstart, lend, last_block,
			expected_data_size, bp, out, outsize,
		)
	{
		return;
	}

	// Which type?
	let btype =
		if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost { BlockType::Uncompressed }
		else if fixed_cost < dynamic_cost { BlockType::Fixed }
		else { BlockType::Dynamic };

	// Save it!
	add_lz77_block(
		btype, last_block, store, lstart, lend,
		expected_data_size, bp, out, outsize,
	);
}

#[allow(unsafe_code)]
/// # Add Dynamic Tree.
///
/// Determine the optimal tree index, then add it to the output.
fn add_dynamic_tree(
	ll_lengths: *const u32,
	d_lengths: *const u32,
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) {
	// Find the index that produces the best size.
	let (i, _) = calculate_tree_size(ll_lengths, d_lengths);

	// Add it!
	unsafe {
		ZopfliEncodeTree(
			ll_lengths,
			d_lengths,
			i & 1,
			i & 2,
			i & 4,
			bp,
			out,
			outsize,
		);
	}
}

#[allow(unsafe_code, clippy::similar_names, clippy::too_many_arguments)]
/// # Add LZ77 Data.
///
/// This adds all lit/len/dist codes from the lists as huffman symbols, but not
/// the end code (256).
fn add_lz77_data(
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	ll_symbols: &[u32; ZOPFLI_NUM_LL],
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_symbols: &[u32; ZOPFLI_NUM_D],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) {
	let mut test_size = 0;
	for i in lstart..lend {
		let dist = unsafe { *store.dists.add(i) };
		let litlen = unsafe { *store.litlens.add(i) };

		// Length only.
		if dist == 0 {
			assert!(litlen < 256);
			unsafe {
				assert!(ll_lengths[litlen as usize] > 0);
				ZopfliAddHuffmanBits(
					ll_symbols[litlen as usize],
					ll_lengths[litlen as usize],
					bp,
					out,
					outsize,
				);
			}
			test_size += 1;
		}
		// Length and distance.
		else {
			// Length first. (Note: we want this bounds check.)
			let (symbol, bits, value) = LENGTH_SYMBOLS_BITS_VALUES[litlen as usize];
			unsafe {
				// Safety: symbols are always in range, but the linked length
				// mustn't be zero.
				assert!(*ll_lengths.get_unchecked(symbol) > 0);
				ZopfliAddHuffmanBits(
					*ll_symbols.get_unchecked(symbol),
					ll_lengths[symbol],
					bp,
					out,
					outsize,
				);
				ZopfliAddBits(u32::from(value), u32::from(bits), bp, out, outsize);
			}

			// Now the distance bits.
			let (symbol, bits, value) = distance_symbol_bits_value(dist);
			unsafe {
				// Safety: symbols are always in range, but the linked length
				// mustn't be zero.
				assert!(*d_lengths.get_unchecked(symbol) > 0);
				ZopfliAddHuffmanBits(
					*d_symbols.get_unchecked(symbol),
					d_lengths[symbol],
					bp,
					out,
					outsize,
				);
				ZopfliAddBits(value, bits, bp, out, outsize);
			}

			test_size += litlen as usize;
		}
	}

	assert!(expected_data_size == 0 || test_size == expected_data_size);
}

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto_type(
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
) -> usize {
	let uncompressed_cost = calculate_block_size(store, lstart, lend, BlockType::Uncompressed);

	// We can skip the expensive fixed-cost calculations for large blocks since
	// they're unlikely ever to use it.
	let fixed_cost =
		if 1000 < store.size { uncompressed_cost }
		else { calculate_block_size(store, lstart, lend, BlockType::Fixed) };

	let dynamic_cost = calculate_block_size(store, lstart, lend, BlockType::Dynamic);

	// If uncompressed is better than everything, return it.
	if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost {
		uncompressed_cost
	}
	// Otherwise choose the smaller of fixed and dynamic.
	else if fixed_cost < dynamic_cost { fixed_cost }
	else { dynamic_cost }
}

/// # Calculate Block Symbol Size w/ Histogram.
fn calculate_block_symbol_size(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
) -> usize {
	if lstart + ZOPFLI_NUM_LL * 3 > lend {
		calculate_block_symbol_size_small(
			ll_lengths,
			d_lengths,
			store,
			lstart,
			lend,
		)
	}
	else {
		let (ll_counts, d_counts) = get_lz77_histogram(store, lstart, lend);
		calculate_block_symbol_size_given_counts(
			&ll_counts,
			&d_counts,
			ll_lengths,
			d_lengths,
			store,
			lstart,
			lend,
		)
	}
}

/// # Calculate Block Symbol Size w/ Histogram.
fn calculate_block_symbol_size_given_counts(
	ll_counts: &[usize; ZOPFLI_NUM_LL],
	d_counts: &[usize; ZOPFLI_NUM_D],
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
) -> usize {
	if lstart + ZOPFLI_NUM_LL * 3 > lend {
		return calculate_block_symbol_size_small(
			ll_lengths,
			d_lengths,
			store,
			lstart,
			lend,
		);
	}

	// The end symbol is always included.
	let mut result = ll_lengths[256] as usize;

	// The early lengths and counts.
	for (ll, lc) in ll_lengths.iter().copied().zip(ll_counts).take(256) {
		result += ll as usize * lc;
	}

	// The lengths and counts with extra bits.
	for (i, lbit) in LENGTH_EXTRA_BITS.iter().copied().enumerate() {
		let i = i + 257;
		result += (ll_lengths[i] + u32::from(lbit)) as usize * ll_counts[i];
	}

	// The distance lengths, counts, and extra bits.
	for (i, dbit) in DISTANCE_EXTRA_BITS.iter().copied().enumerate().take(30) {
		result += (d_lengths[i] + u32::from(dbit)) as usize * d_counts[i];
	}

	result
}

#[allow(unsafe_code, clippy::similar_names)]
/// # Calculate Small Block Symbol Size.
fn calculate_block_symbol_size_small(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
) -> usize {
	// The end symbol is always included.
	let mut result = ll_lengths[256] as usize;

	// Loop the store if we have data to loop.
	if lstart < lend {
		// Make sure the end does not exceed the store!
		assert!(lend <= store.size);
		for i in lstart..lend {
			let lz77_litlens = unsafe { *store.litlens.add(i) };
			assert!(lz77_litlens < 259);

			let lz77_dists = unsafe { *store.dists.add(i) };
			if lz77_dists == 0 {
				result += ll_lengths[lz77_litlens as usize] as usize;
			}
			else {
				let (lsym, lbits, _) = LENGTH_SYMBOLS_BITS_VALUES[lz77_litlens as usize];
				// Safety: the length symbols are always in range.
				result += unsafe { *ll_lengths.get_unchecked(lsym) } as usize;
				result += usize::from(lbits);

				let dsym = distance_symbol(u32::from(lz77_dists));
				result += usize::from(DISTANCE_EXTRA_BITS[dsym]);
				result += d_lengths[dsym] as usize;
			}
		}
	}

	result
}

#[allow(unsafe_code)]
/// # Calculate the Exact Tree Size (in Bits).
///
/// This returns the index that produced the smallest size, and its size.
///
/// This is a rewrite of the original `deflate.c` method.
fn calculate_tree_size(ll_lengths: *const u32, d_lengths: *const u32) -> (i32, usize) {
	let mut best_size = 0;
	let mut best_idx = 0;

	for i in 0..8 {
		let size = unsafe {
			ZopfliEncodeTree(
				ll_lengths,
				d_lengths,
				i & 1,
				i & 2,
				i & 4,
				std::ptr::null_mut(),
				std::ptr::null_mut(),
				std::ptr::null_mut(),
			)
		};
		if best_size == 0 || size < best_size {
			best_size = size;
			best_idx = i;
		}
	}

	(best_idx, best_size)
}

#[allow(clippy::cast_possible_truncation)]
/// # Distance Symbol, Extra Bits, and Bits Value.
///
/// Calculate the symbol and bits given the distance. There is unfortunately
/// too much variation to justify a simple table like the one used for lengths;
/// (compiler-optimized) math is our best bet.
const fn distance_symbol_bits_value(dist: u16) -> (usize, u32, u32) {
	if dist < 5 { (dist.saturating_sub(1) as usize, 0, 0) }
	else {
		let d_log = (dist - 1).ilog2();

		let r = ((dist as u32 - 1) >> (d_log - 1)) & 1;
		let sym = (d_log * 2 + r) as usize;

		let (m1, m2) = DISTANCE_EXTRA_BITS_MASK[d_log as usize];
		let value = (dist as u32 - m1) & m2;

		(sym, d_log - 1, value)
	}
}

#[allow(clippy::cast_precision_loss)]
/// # Estimate Block Cost (in Bits).
///
/// Return the estimated size to encode the tree and all literal, length, and
/// distance symbols (plus their extra bits).
fn estimate_cost(store: &ZopfliLZ77Store, lstart: usize, lend: usize) -> f64 {
	calculate_block_size_auto_type(store, lstart, lend) as f64
}

#[allow(clippy::similar_names)]
/// # Find Largest Splittable Block.
///
/// This finds the largest available block for splitting, evenly spreading the
/// load if a limited number of blocks are requested.
///
/// Returns `false` if no blocks are found.
fn find_largest_splittable_block(
	lz77size: usize,
	done: &HashSet<usize, NoHash>,
	splitpoints: &[usize],
	lstart: &mut usize,
	lend: &mut usize,
) -> bool {
	let mut best = 0;
	for i in 0..=splitpoints.len() {
		let start =
			if i == 0 { 0 }
			else { splitpoints[i - 1] };
		let end =
			if i < splitpoints.len() { splitpoints[i] }
			else { lz77size - 1 };

		// We found a match!
		if best < end - start && ! done.contains(&start) {
			*lstart = start;
			*lend = end;
			best = end - start;
		}
	}
	MINIMUM_SPLIT_DISTANCE <= best
}

/// # Minimum Split Cost.
///
/// Return the index of the smallest split cost between `start..end`.
fn find_minimum_cost(
	store: &ZopfliLZ77Store,
	mut start: usize,
	mut end: usize,
) -> (usize, f64) {
	// Keep track of the original start/end points.
	let split_start = start - 1;
	let split_end = end;

	let mut best_cost = f64::INFINITY;
	let mut best_idx = start;

	// Small chunks don't need much.
	if end - start < 1024 {
		for i in start..end {
			let cost = split_cost(store, split_start, i, split_end);
			if cost < best_cost {
				best_cost = cost;
				best_idx = i;
			}
		}
		return (best_idx, best_cost);
	}

	// Divide and conquer.
	let mut p = [0_usize; MINIMUM_SPLIT_DISTANCE - 1];
	let mut last_best_cost = f64::INFINITY;
	while MINIMUM_SPLIT_DISTANCE <= end - start {
		let mut best_p_idx = 0;
		for (i, pp) in p.iter_mut().enumerate() {
			*pp = start + (i + 1) * ((end - start).wrapping_div(MINIMUM_SPLIT_DISTANCE));
			let line_cost =
				if best_idx == *pp { last_best_cost }
				else { split_cost(store, split_start, *pp, split_end) };

			if i == 0 || line_cost < best_cost {
				best_cost = line_cost;
				best_p_idx = i;
			}
		}

		// No improvement; we're done.
		if last_best_cost < best_cost { break; }

		// Nudge the boundaries and back again.
		best_idx = p[best_p_idx];
		if 0 < best_p_idx { start = p[best_p_idx - 1]; }
		if best_p_idx + 1 < p.len() { end = p[best_p_idx + 1]; }

		last_best_cost = best_cost;
	}

	(best_idx, last_best_cost)
}

/// # Calculate the Bit Lengths for Dynamic Block Symbols.
///
/// This chooses lengths that lead to the smallest tree/symbol encoding.
/// (This is not necessarily the optimal Huffman lengths.)
///
/// The total size in bits (minus the 3-bit header) is returned.
///
/// This is a rewrite of the original `deflate.c` method.
fn get_dynamic_lengths(
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	ll_lengths: &mut [u32; ZOPFLI_NUM_LL],
	d_lengths: &mut [u32; ZOPFLI_NUM_D],
) -> usize {
	// Populate some counts.
	let (mut ll_counts, d_counts) = get_lz77_histogram(store, lstart, lend);
	ll_counts[256] = 1;

	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_LL>(&ll_counts, ll_lengths.as_mut_ptr());
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_D>(&d_counts, d_lengths.as_mut_ptr());

	patch_distance_codes(d_lengths);
	try_optimize_huffman_for_rle(
		store,
		lstart,
		lend,
		&ll_counts,
		&d_counts,
		ll_lengths,
		d_lengths,
	)
}

#[allow(unsafe_code)]
/// # Symbol Spans in Raw Bytes.
fn get_lz77_byte_range(
	store: &ZopfliLZ77Store,
	lstart: usize,
	mut lend: usize,
) -> usize {
	if lstart >= lend { 0 }
	else {
		lend -= 1;
		unsafe {
			let length =
				if 0 == *store.dists.add(lend) { 1 }
				else { usize::from(*store.litlens.add(lend)) };
			length + *store.pos.add(lend) - *store.pos.add(lstart)
		}
	}
}

#[allow(unsafe_code)]
/// # LZ77 Histogram.
///
/// Return populated length and distance counts for the symbols in a given
/// range.
fn get_lz77_histogram(store: &ZopfliLZ77Store, lstart: usize, lend: usize)
-> ([usize; ZOPFLI_NUM_LL], [usize; ZOPFLI_NUM_D]) {
	// Count the symbols directly.
	if lstart + ZOPFLI_NUM_LL * 3 > lend {
		let mut ll_counts = [0_usize; ZOPFLI_NUM_LL];
		let mut d_counts = [0_usize; ZOPFLI_NUM_D];

		for i in lstart..lend {
			// Safety: the symbols are always in range.
			unsafe {
				*ll_counts.get_unchecked_mut(*store.ll_symbol.add(i) as usize) += 1;
				if *store.dists.add(i) != 0 {
					*d_counts.get_unchecked_mut(*store.d_symbol.add(i) as usize) += 1;
				}
			}
		}

		(ll_counts, d_counts)
	}
	// Subtract the cumulative histograms at the start from the end to get the
	// one for this range.
	else {
		let (mut ll_counts, mut d_counts) = _get_lz77_histogram(store, lend - 1);
		if 0 < lstart {
			let (ll_counts2, d_counts2) = _get_lz77_histogram(store, lstart - 1);

			for (a, b) in ll_counts.iter_mut().zip(ll_counts2) { *a -= b; }
			for (a, b) in d_counts.iter_mut().zip(d_counts2) { *a -= b; }
		}

		(ll_counts, d_counts)
	}
}

#[allow(unsafe_code, clippy::similar_names)]
fn _get_lz77_histogram(store: &ZopfliLZ77Store, pos: usize)
-> ([usize; ZOPFLI_NUM_LL], [usize; ZOPFLI_NUM_D]) {
	// The relative chunked positions.
	let llpos = ZOPFLI_NUM_LL * pos.wrapping_div(ZOPFLI_NUM_LL);
	let dpos = ZOPFLI_NUM_D * pos.wrapping_div(ZOPFLI_NUM_D);

	// Start by copying the counts directly from the nearest chunk.
	let mut ll_counts: [usize; ZOPFLI_NUM_LL] = unsafe {
		*store.ll_counts.add(llpos).cast()
	};
	let mut d_counts: [usize; ZOPFLI_NUM_D]  = unsafe {
		*store.d_counts.add(dpos).cast()
	};

	// Subtract the symbols occurences from the offset.
	for i in pos + 1..store.size.min(llpos + ZOPFLI_NUM_LL) {
		unsafe {
			// Safety: symbols are always in range, and store always has
			// store.size symbol entries.
			let idx = *store.ll_symbol.add(i) as usize;
			*ll_counts.get_unchecked_mut(idx) -= 1;
		}
	}
	for i in pos + 1..store.size.min(dpos + ZOPFLI_NUM_D) {
		unsafe {
			// Safety: symbols are always in range, and store always has
			// store.size symbol entries.
			if *store.dists.add(i) != 0 {
				let idx = *store.d_symbol.add(i) as usize;
				*d_counts.get_unchecked_mut(idx) -= 1;
			}
		}
	}

	(ll_counts, d_counts)
}

/// # Split Block Cost.
///
/// Return the sum of the estimated costs of the left and right sections of the
/// data.
fn split_cost(store: &ZopfliLZ77Store, start: usize, mid: usize, end: usize) -> f64 {
	estimate_cost(store, start, mid) + estimate_cost(store, mid, end)
}

#[allow(unsafe_code, clippy::integer_division, clippy::cast_sign_loss)]
/// # Optimize Huffman RLE Compression.
///
/// Change the population counts to improve Huffman tree compression,
/// particularly its RLE part.
///
/// This is a rewrite of the original `deflate.c` method.
fn optimize_huffman_for_rle(mut counts: &mut [usize]) {
	// Convert counts to a proper slice with trailing zeroes trimmed.
	let ptr = counts.as_mut_ptr();
	while let [ rest @ .., 0 ] = counts { counts = rest; }
	if counts.is_empty() { return; }

	// Find collapseable ranges!
	let mut stride = 0;
	let mut scratch = counts[0];
	let mut sum = 0;
	let four = counts.len().saturating_sub(3);
	for (i, (&count, good)) in counts.iter().zip(GoodForRle::new(counts)).enumerate() {
		// Time to reset (and maybe collapse).
		if good || count.abs_diff(scratch) >= 4 {
			// Collapse the stride if it is as least four and contained
			// something non-zero.
			if sum != 0 && stride >= 4 {
				let v = ((sum + stride / 2) / stride).max(1);
				// Safety: this is a very un-Rust thing to do, but we're only
				// modifying values after-the-fact; the current and future
				// data remains as it was.
				unsafe {
					for j in i - stride..i { ptr.add(j).write(v); }
				}
			}

			// Reset!
			stride = 0;
			sum = 0;

			// If we have at least four remaining values (including the
			// current), take a sort of weighted average of them.
			if i < four {
				scratch = (
					unsafe { *counts.get_unchecked(i + 3) } +
					counts[i + 2] +
					counts[i + 1] +
					count +
					2
				) / 4;
			}
			// Otherwise just use the current value.
			else { scratch = count; }
		}

		stride += 1;
		sum += count;
	}

	// Collapse the trailing stride, if any.
	if sum != 0 && stride >= 4 {
		let v = ((sum + stride / 2) / stride).max(1);
		let len = counts.len();
		counts[len - stride..].fill(v);
	}
}

/// # Patch Buggy Distance Codes.
///
/// Ensure there are at least two distance codes to avoid issues with buggy
/// decoders.
///
/// Note: `d_lengths` has a fixed length of 32.
fn patch_distance_codes(d_lengths: &mut [u32; ZOPFLI_NUM_D]) {
	let mut one: Option<usize> = None;
	for (i, dist) in d_lengths.iter().copied().enumerate().take(30) {
		// We have (at least) two non-zero entries; no patching needed!
		if 0 != dist && one.replace(i).is_some() { return; }
	}

	match one {
		// The first entry had a code, so patch the second to give us two.
		Some(0) => { d_lengths[1] = 1; },
		// Patch the first entry to give us two.
		Some(_) => { d_lengths[0] = 1; },
		// There were no codes, so let's just patch the first two.
		None => {
			d_lengths[0] = 1;
			d_lengths[1] = 1;
		},
	}
}

#[allow(unsafe_code, clippy::too_many_arguments)]
/// # (Maybe) Add LZ77 Expensive Fixed Block.
///
/// This runs the full suite of fixed-tree tests on the data and writes it to
/// the output if it is indeed better than the uncompressed/dynamic variants.
///
/// Returns `true` if data is written.
fn try_lz77_expensive_fixed(
	store: &ZopfliLZ77Store,
	uncompressed_cost: usize,
	dynamic_cost: usize,
	lstart: usize,
	lend: usize,
	last_block: bool,
	expected_data_size: usize,
	bp: *mut u8,
	out: *mut *mut u8,
	outsize: *mut usize,
) -> bool {
	let mut fixed_store = LZ77Store::new(store.data);
	let instart = unsafe { *store.pos.add(lstart) };
	let inend = instart + get_lz77_byte_range(store, lstart, lend);
	let blocksize = inend - instart;

	// Run all the expensive fixed-cost checks.
	CACHE.with_borrow_mut(|c| c.init(blocksize));
	SQUEEZE.with_borrow_mut(|s| {
		s.init(blocksize + 1);
		s.reset_costs();

		// Pull the hasher.
		HASH.with_borrow_mut(|h| {
			// Get the cost.
			let cost = h.get_best_lengths(
				store.data,
				instart,
				inend,
				None,
				s.costs.as_mut_slice(),
			);
			debug_assert!(cost < 1E30);

			// Trace backwards and forwards.
			s.trace_paths();
			h.follow_paths(
				store.data,
				instart,
				inend,
				s.paths.as_slice(),
				&mut fixed_store,
			);
		});
	});

	// Find the resulting cost.
	let fixed_cost = calculate_block_size(
		&fixed_store,
		0,
		fixed_store.size,
		BlockType::Fixed,
	);

	// If it is better than dynamic, and uncompressed isn't better than both
	// fixed and dynamic, it's the best and worth writing!
	if fixed_cost < dynamic_cost && (fixed_cost <= uncompressed_cost || dynamic_cost <= uncompressed_cost) {
		add_lz77_block(
			BlockType::Fixed, last_block, &fixed_store, 0, fixed_store.size,
			expected_data_size, bp, out, outsize,
		);
		true
	}
	else { false }
}

/// # Try Huffman RLE Optimization.
///
/// This method attempts to optimize the RLE parts of the block, saving the
/// result if better, ignoring it if not.
///
/// The size of the encoded tree and data (in bits) is returned, minus the
/// 3-bit block header.
///
/// This is a rewrite of the original `deflate.c` method.
fn try_optimize_huffman_for_rle(
	store: &ZopfliLZ77Store,
	lstart: usize,
	lend: usize,
	ll_counts: &[usize; ZOPFLI_NUM_LL],
	d_counts: &[usize; ZOPFLI_NUM_D],
	ll_lengths: &mut [u32; ZOPFLI_NUM_LL],
	d_lengths: &mut [u32; ZOPFLI_NUM_D],
) -> usize {
	let (_, treesize) = calculate_tree_size(ll_lengths.as_ptr(), d_lengths.as_ptr());

	let datasize = calculate_block_symbol_size_given_counts(
		ll_counts,
		d_counts,
		ll_lengths,
		d_lengths,
		store,
		lstart,
		lend,
	);

	// Copy the counts, optimize them, etc., etc.
	let mut ll_lengths2 = [0_u32; ZOPFLI_NUM_LL];
	let mut d_lengths2 = [0_u32; ZOPFLI_NUM_D];
	let mut ll_counts2 = *ll_counts;
	let mut d_counts2 = *d_counts;
	optimize_huffman_for_rle(&mut ll_counts2);
	optimize_huffman_for_rle(&mut d_counts2);
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_LL>(&ll_counts2, ll_lengths2.as_mut_ptr());
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_D>(&d_counts2, d_lengths2.as_mut_ptr());
	patch_distance_codes(&mut d_lengths2);

	let (_, treesize2) = calculate_tree_size(ll_lengths2.as_ptr(), d_lengths2.as_ptr());
	let datasize2 = calculate_block_symbol_size_given_counts(
		ll_counts,
		d_counts,
		&ll_lengths2,
		&d_lengths2,
		store,
		lstart,
		lend,
	);

	let sum = treesize + datasize;
	let sum2 = treesize2 + datasize2;
	if sum <= sum2 { sum }
	else {
		ll_lengths.copy_from_slice(ll_lengths2.as_slice());
		d_lengths.copy_from_slice(d_lengths2.as_slice());
		sum2
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_good_for_rle() {
		const COUNTS1: &[usize] = &[196, 23, 10, 12, 5, 4, 1, 23, 8, 2, 6, 5, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 47, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 8, 1, 1, 1, 0, 4, 7, 2, 0, 1, 10, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 42, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 44, 284, 1, 15, 21, 0, 55, 0, 19, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 320, 12, 0, 0, 17, 3, 0, 3, 2];
		const COUNTS2: &[usize] = &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 0, 288, 11, 41, 6, 5, 2, 0, 0, 0, 1];
		const COUNTS3: &[usize] = &[201, 24, 10, 12, 5, 4, 1, 24, 8, 2, 6, 4, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 49, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 9, 1, 1, 1, 0, 4, 6, 2, 0, 1, 8, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 44, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 46, 281, 1, 20, 3, 10, 59, 0, 4, 12, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 318, 12, 0, 0, 21, 0, 0, 3, 2];

		for c in [COUNTS1, COUNTS2, COUNTS3] {
			// Make sure our ExactSizeness is working.
			let good = GoodForRle::new(c);
			assert_eq!(
				good.len(),
				c.len(),
				"GoodForRle iterator count does not match source.",
			);

			// And make sure that is the count we actually end up with.
			let good = good.collect::<Vec<bool>>();
			assert_eq!(
				good.len(),
				c.len(),
				"Collected GoodForRle iterator count does not match source.",
			);
		}
	}
}
