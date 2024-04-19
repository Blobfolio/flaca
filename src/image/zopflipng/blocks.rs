/*!
# Flaca: Zopflipng Blocks.

This module contains the deflate entrypoint and all of the block-related odds
and ends that didn't make it into other modules.
*/

use dactyl::NoHash;
use std::collections::HashSet;
use super::{
	DEFLATE_ORDER,
	DISTANCE_BITS,
	DISTANCE_VALUES,
	FIXED_TREE_D,
	FIXED_TREE_LL,
	LENGTH_SYMBOLS_BITS_VALUES,
	LZ77Store,
	stats::{
		RanState,
		SymbolStats,
	},
	zopfli_error,
	zopfli_length_limited_code_lengths,
	zopfli_lengths_to_symbols,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
	ZopfliOut,
	ZopfliState,
};



/// # Length Symbol Extra Bits.
const LENGTH_EXTRA_BITS: [u8; 29] = [
	0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2,
	3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];

/// # Minimum Split Distance.
const MINIMUM_SPLIT_DISTANCE: usize = 10;

/// # Max Split Points.
const MAX_SPLIT_POINTS: usize = 14;



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
	#[inline]
	/// # Uncompressed Split Pass.
	///
	/// This sets the uncompressed split points, by way of first setting the
	/// LZ77 split points.
	///
	/// In terms of order-of-operations, this must be called _before_ the
	/// second-stage LZ77 pass as it would otherwise blow away that data.
	fn split_raw(&mut self, arr: &[u8], instart: usize, state: &mut ZopfliState, store: &mut LZ77Store)
	-> Result<usize, ZopfliError> {
		// Populate an LZ77 store from a greedy pass. This results in better
		// block choices than a full optimal pass.
		state.greedy(arr, instart, store, None)?;

		// Do an LZ77 pass.
		let len = self.split_lz77(store)?;

		// Find the corresponding uncompressed positions.
		if 0 < len && len <= MAX_SPLIT_POINTS {
			let mut pos = instart;
			let mut j = 0;
			for (i, e) in store.entries.iter().enumerate().take(self.slice2[len - 1] + 1) {
				if i == self.slice2[j] {
					self.slice1[j] = pos;
					j += 1;
					if j == len { return Ok(len); }
				}
				pos += e.length() as usize;
			}

			unreachable!();
		}
		else { Ok(len) }
	}

	/// # LZ77 Split Pass.
	///
	/// This sets the LZ77 split points according to convoluted cost
	/// evaluations.
	fn split_lz77(&mut self, store: &LZ77Store) -> Result<usize, ZopfliError> {
		// This won't work on tiny files.
		if store.len() < MINIMUM_SPLIT_DISTANCE { return Ok(0); }

		// Get started!
		self.done.clear();
		let mut lstart = 0;
		let mut lend = store.len();
		let mut last = 0;
		let mut len = 0;
		loop {
			let (llpos, llcost) = find_minimum_cost(store, lstart + 1, lend)?;
			if llpos <= lstart || llpos >= lend {
				return Err(zopfli_error!());
			}

			// Ignore points we've already covered.
			if llpos == lstart + 1 || calculate_block_size_auto_type(store, lstart, lend)? < llcost {
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
				store.len(),
				&self.done,
				&self.slice2[..len],
				&mut lstart,
				&mut lend,
			) { break; }
		}

		Ok(len)
	}

	#[allow(unsafe_code)]
	#[inline]
	/// # Split Best.
	///
	/// Compare the optimal raw split points with a dedicated lz77 pass and
	/// return whichever is predicted to compress better.
	fn split(
		&mut self,
		numiterations: i32,
		arr: &[u8],
		instart: usize,
		store: &mut LZ77Store,
		store2: &mut LZ77Store,
		state: &mut ZopfliState,
	) -> Result<&[usize], ZopfliError> {
		// Start by splitting uncompressed.
		let limit = self.split_raw(arr, instart, state, store2)?.min(MAX_SPLIT_POINTS);

		let mut cost1 = 0;
		let mut store3 = LZ77Store::new();
		for i in 0..=limit {
			let start = if i == 0 { instart } else { self.slice1[i - 1] };
			let end = if i < limit { self.slice1[i] } else { arr.len() };

			// This assertion is redundant as we explicitly check range sanity
			// earlier and later in the pipeline.
			debug_assert!(start <= end && end <= arr.len());

			// Make another store.
			store2.clear();
			lz77_optimal(
				// Safety: the split points are checked at creation.
				unsafe { arr.get_unchecked(..end) },
				start,
				numiterations,
				store2,
				&mut store3,
				state,
			)?;
			cost1 += calculate_block_size_auto_type(store2, 0, store2.len())?;

			// Append its data to our main store.
			store.append(store2);

			// Save the chunk size to our best.
			if i < limit { self.slice2[i] = store.len(); }
		}

		// Try a second pass, recalculating the LZ77 splits with the updated
		// store details.
		if 1 < limit {
			// Move slice2 over to slice1 so we can repopulate slice2.
			self.slice1.copy_from_slice(self.slice2.as_slice());

			let limit2 = self.split_lz77(store)?.min(MAX_SPLIT_POINTS);
			let mut cost2 = 0;
			for i in 0..=limit2 {
				let start = if i == 0 { 0 } else { self.slice2[i - 1] };
				let end = if i < limit2 { self.slice2[i] } else { store.len() };
				cost2 += calculate_block_size_auto_type(store, start, end)?;
			}

			// It's better!
			if cost2 < cost1 { Ok(&self.slice2[..limit2]) }
			else { Ok(&self.slice1[..limit]) }
		}
		else { Ok(&self.slice2[..limit]) }
	}
}



/// # Deflate a Part.
///
/// Image compression is done in chunks of a million bytes. This does all the
/// work there is to do for one such chunk.
///
/// More specifically, this explores different possible split points for the
/// chunk, then writes the resulting blocks to the output file.
pub(crate) fn deflate_part(
	state: &mut ZopfliState,
	splits: &mut SplitPoints,
	numiterations: i32,
	last_block: bool,
	arr: &[u8],
	instart: usize,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Find the split points.
	let mut store = LZ77Store::new();
	let mut store2 = LZ77Store::new();
	let best = splits.split(
		numiterations,
		arr,
		instart,
		&mut store,
		&mut store2,
		state,
	)?;

	// Write the data!
	for i in 0..=best.len() {
		let start = if i == 0 { 0 } else { best[i - 1] };
		let end = if i < best.len() { best[i] } else { store.len() };
		add_lz77_block_auto_type(
			i == best.len() && last_block,
			&store,
			&mut store2,
			state,
			arr,
			start,
			end,
			0,
			out,
		)?;
	}

	Ok(())
}



#[repr(u8)]
#[derive(Clone, Copy, Eq, PartialEq)]
/// # Block Type.
///
/// This enum is mainly used to specify the type of block size to calculate.
enum BlockType {
	Uncompressed = 0_u8,
	Fixed = 1_u8,
	Dynamic = 2_u8,
}



/// # RLE-Optimized Stretches.
///
/// This iterator yields a boolean value for each entry of the source slice,
/// `true` for distance codes in a sequence of 5+ zeroes or 7+ (identical)
/// non-zeroes, `false` otherwise.
///
/// It moots the need to collect such values into a vector in advance,
/// reducing the number of passes required to optimize Huffman codes.
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
			// Note the reptition and circle back around. This will always
			// trigger on the first pass, so stride will always be at least
			// one.
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



#[allow(clippy::too_many_arguments)]
/// # Add LZ77 Block.
///
/// Add a deflate block with the given LZ77 data to the output.
fn add_lz77_block(
	btype: BlockType,
	last_block: bool,
	store: &LZ77Store,
	arr: &[u8],
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Uncompressed blocks are easy!
	if matches!(btype, BlockType::Uncompressed) {
		let (instart, inend) = get_lz77_byte_range(store, lstart, lend)?;
		out.add_uncompressed_block(last_block, arr.as_ptr(), instart, inend);
		return Ok(());
	}

	// Add some bits.
	out.add_bit(i32::from(last_block));
	out.add_bit((btype as i32) & 1);
	out.add_bit(((btype as i32) & 2) >> 1);

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
			)?;
			add_dynamic_tree(&ll_lengths, &d_lengths, out)?;
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
		out
	)?;

	// Finish up by writting the end symbol.
	out.add_huffman_bits(ll_symbols[256], ll_lengths[256]);
	Ok(())
}

#[allow(
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::too_many_arguments,
)]
#[inline]
/// # Add LZ77 Block (Automatic Type).
///
/// This calculates the expected output sizes for all three block types, then
/// writes the best one to the output file.
fn add_lz77_block_auto_type(
	last_block: bool,
	store: &LZ77Store,
	fixed_store: &mut LZ77Store,
	state: &mut ZopfliState,
	arr: &[u8],
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	// If the block is empty, we can assume a fixed-tree layout.
	if lstart >= lend {
		out.add_bits(u32::from(last_block), 1);
		out.add_bits(1, 2);
		out.add_bits(0, 7);
		return Ok(());
	}

	// Calculate the three costs.
	let uncompressed_cost = calculate_block_size(store, lstart, lend, BlockType::Uncompressed)?;
	let fixed_cost = calculate_block_size(store, lstart, lend, BlockType::Fixed)?;
	let dynamic_cost = calculate_block_size(store, lstart, lend, BlockType::Dynamic)?;

	// Fixed stores are only useful up to a point; we can skip the overhead
	// if the store is big or the dynamic cost estimate is unimpressive.
	if
		(store.len() < 1000 || (fixed_cost as f64) <= (dynamic_cost as f64) * 1.1) &&
		try_lz77_expensive_fixed(
			store, fixed_store, state, uncompressed_cost, dynamic_cost,
			arr, lstart, lend, last_block,
			expected_data_size, out,
		)?
	{
		return Ok(());
	}

	// Which type?
	let btype =
		if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost { BlockType::Uncompressed }
		else if fixed_cost < dynamic_cost { BlockType::Fixed }
		else { BlockType::Dynamic };

	// Save it!
	add_lz77_block(
		btype, last_block, store, arr, lstart, lend,
		expected_data_size, out,
	)
}

#[inline(never)]
/// # Add Dynamic Tree.
///
/// Determine the optimal tree index, then add it to the output.
fn add_dynamic_tree(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	let (i, _) = calculate_tree_size(ll_lengths, d_lengths)?;
	encode_tree(ll_lengths, d_lengths, i, Some(out))?;
	Ok(())
}

#[allow(
	clippy::cast_sign_loss,
	clippy::too_many_arguments,
)]
/// # Add LZ77 Data.
///
/// This adds all lit/len/dist codes from the lists as huffman symbols, but not
/// the end code (256).
fn add_lz77_data(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	expected_data_size: usize,
	ll_symbols: &[u32; ZOPFLI_NUM_LL],
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_symbols: &[u32; ZOPFLI_NUM_D],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	let mut test_size = 0;
	for e in &store.entries[lstart..lend] {
		// Length only.
		if e.dist <= 0 {
			if (e.litlen as u16) >= 256 {
				return Err(zopfli_error!());
			}
			if ll_lengths[e.litlen as usize] == 0 { return Err(zopfli_error!()); }

			out.add_huffman_bits(
				ll_symbols[e.litlen as usize],
				ll_lengths[e.litlen as usize],
			);
			test_size += 1;
		}
		// Length and distance.
		else {
			let (symbol, bits, value) = LENGTH_SYMBOLS_BITS_VALUES[e.litlen as usize];
			if ll_lengths[symbol as usize] == 0 { return Err(zopfli_error!()); }

			out.add_huffman_bits(
				ll_symbols[symbol as usize],
				ll_lengths[symbol as usize],
			);
			out.add_bits(u32::from(value), u32::from(bits));

			// Now the distance bits.
			if d_lengths[e.d_symbol as usize] == 0 { return Err(zopfli_error!()); }
			out.add_huffman_bits(
				d_symbols[e.d_symbol as usize],
				d_lengths[e.d_symbol as usize],
			);
			out.add_bits(
				u32::from(DISTANCE_VALUES[e.dist as usize]),
				u32::from(DISTANCE_BITS[e.d_symbol as usize]),
			);

			test_size += e.litlen as usize;
		}
	}

	if expected_data_size == 0 || test_size == expected_data_size { Ok(()) }
	else { Err(zopfli_error!()) }
}

/// # Calculate Block Size (in Bits).
fn calculate_block_size(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	btype: BlockType,
) -> Result<usize, ZopfliError> {
	match btype {
		BlockType::Uncompressed => {
			let (instart, inend) = get_lz77_byte_range(store, lstart, lend)?;
			let blocksize = inend - instart;

			// Blocks larger than u16::MAX need to be split.
			let blocks = blocksize.div_ceil(65_535);
			Ok(blocks * 40 + blocksize * 8)
		},
		BlockType::Fixed =>
			Ok(calculate_block_symbol_size(
				&FIXED_TREE_LL,
				&FIXED_TREE_D,
				store,
				lstart,
				lend,
			)? + 3),
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

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto_type(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<usize, ZopfliError> {
	let uncompressed_cost = calculate_block_size(store, lstart, lend, BlockType::Uncompressed)?;

	// We can skip the expensive fixed-cost calculations for large blocks since
	// they're unlikely ever to use it.
	let fixed_cost =
		if 1000 < store.len() { uncompressed_cost }
		else { calculate_block_size(store, lstart, lend, BlockType::Fixed)? };

	let dynamic_cost = calculate_block_size(store, lstart, lend, BlockType::Dynamic)?;

	// If uncompressed is better than everything, return it.
	if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost {
		Ok(uncompressed_cost)
	}
	// Otherwise choose the smaller of fixed and dynamic.
	else if fixed_cost < dynamic_cost { Ok(fixed_cost) }
	else { Ok(dynamic_cost) }
}

/// # Calculate Block Symbol Size w/ Histogram.
fn calculate_block_symbol_size(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<usize, ZopfliError> {
	if lstart + ZOPFLI_NUM_LL * 3 > lend {
		Ok(calculate_block_symbol_size_small(
			ll_lengths,
			d_lengths,
			store,
			lstart,
			lend,
		))
	}
	else {
		let (ll_counts, d_counts) = store.histogram(lstart, lend)?;
		Ok(calculate_block_symbol_size_given_counts(
			&ll_counts,
			&d_counts,
			ll_lengths,
			d_lengths,
			store,
			lstart,
			lend,
		))
	}
}

/// # Calculate Block Symbol Size w/ Histogram and Counts.
fn calculate_block_symbol_size_given_counts(
	ll_counts: &[usize; ZOPFLI_NUM_LL],
	d_counts: &[usize; ZOPFLI_NUM_D],
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &LZ77Store,
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
	for (i, dbit) in DISTANCE_BITS.iter().copied().enumerate().take(30) {
		result += (d_lengths[i] + u32::from(dbit)) as usize * d_counts[i];
	}

	result
}

/// # Calculate Small Block Symbol Size.
fn calculate_block_symbol_size_small(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> usize {
	// The end symbol is always included.
	let mut result = ll_lengths[256] as usize;

	// Loop the store if we have data to loop.
	if lstart < lend {
		// Make sure the end does not exceed the store!
		for e in &store.entries[lstart..lend] {
			if e.dist <= 0 {
				result += ll_lengths[e.litlen as usize] as usize;
			}
			else {
				result += LENGTH_SYMBOLS_BITS_VALUES[e.litlen as usize].1 as usize;
				result += ll_lengths[e.ll_symbol as usize] as usize;
				result += DISTANCE_BITS[e.d_symbol as usize] as usize;
				result += d_lengths[e.d_symbol as usize] as usize;
			}
		}
	}

	result
}

/// # Calculate the Exact Tree Size (in Bits).
///
/// This returns the index that produced the smallest size, and its size.
fn calculate_tree_size(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
) -> Result<(u8, usize), ZopfliError> {
	let mut best_size = 0;
	let mut best_idx = 0;

	for i in 0..8 {
		let size = encode_tree(ll_lengths, d_lengths, i, None)?;

		if best_size == 0 || size < best_size {
			best_size = size;
			best_idx = i;
		}
	}

	Ok((best_idx, best_size))
}

#[allow(
	clippy::cast_possible_truncation,
	clippy::cognitive_complexity, // Yeah, this is terrible!
	clippy::similar_names,
)]
#[inline(never)]
/// # Encode Huffman Tree.
///
/// Build a tree and optionally write it to the output file, returning the
/// resulting size in any case.
///
/// The `use_16`, et al, variables indicate whether or not to encode
/// eligible counts in the extended parts of the alphabet.
fn encode_tree(
	ll_lengths: &[u32; ZOPFLI_NUM_LL],
	d_lengths: &[u32; ZOPFLI_NUM_D],
	extra: u8,
	out: Option<&mut ZopfliOut>,
) -> Result<usize, ZopfliError> {
	// To use or not to use the extended part of the alphabet.
	let use_16: bool = 0 != extra & 1;
	let use_17: bool = 0 != extra & 2;
	let use_18: bool = 0 != extra & 4;

	// Repetition counts and bit values. Only applicable when `out.is_some()`.
	let mut rle: Vec<(u32, u32)> = Vec::new();

	// Code counts and lengths.
	let mut cl_counts = [0_usize; 19];
	let mut cl_lengths = [0_u32; 19];

	// Find the last non-zero length index.
	let mut hlit = 29;
	while hlit > 0 && ll_lengths[256 + hlit] == 0 { hlit -= 1; }
	let hlit2 = hlit + 257; // Same as hlit, but in symbol form.

	// And do the same for distance.
	let mut hdist = 29;
	while hdist > 0 && d_lengths[hdist] == 0 { hdist -= 1; }

	// The upcoming loop processes lengths and distances together (one after
	// the other). This macro returns the symbol from the appropriate table
	// based on the index.
	macro_rules! length_or_distance {
		($idx:ident) => (
			if $idx < hlit2 { ll_lengths[$idx] }
			else {
				// This mask isn't necessary but the compiler doesn't
				// understand what we're trying to do.
				d_lengths[($idx - hlit2).min(29)]
			}
		);
	}

	// Run through all the length symbols, then the distance symbols, with the
	// odd skip to keep us on our toes.
	let lld_total = hlit2 + hdist + 1;
	let mut i = 0;
	while i < lld_total {
		let mut count = 1;
		let symbol = length_or_distance!(i);
		if symbol >= 19 { return Err(zopfli_error!()); }

		// Peek ahead; we may be able to do more in one go.
		if use_16 || (symbol == 0 && (use_17 || use_18)) {
			let mut j = i + 1;
			while j < lld_total && symbol == length_or_distance!(j) {
				count += 1;
				j += 1;
			}

			// Skip these indices, if any, on the next pass.
			i += count - 1;
		}

		// Repetitions of zeroes.
		if symbol == 0 && count >= 3 {
			if use_18 {
				while count >= 11 {
					let count2 = count.min(138);
					if out.is_some() { rle.push((18, count2 as u32 - 11)); }
					cl_counts[18] += 1;
					count -= count2;
				}
			}
			if use_17 {
				while count >= 3 {
					let count2 = count.min(10);
					if out.is_some() { rle.push((17, count2 as u32 - 3)); }
					cl_counts[17] += 1;
					count -= count2;
				}
			}
		}

		// Repetitions of any symbol.
		if use_16 && count >= 4 {
			// The first one always counts.
			count -= 1;
			cl_counts[symbol as usize] += 1;
			if out.is_some() { rle.push((symbol, 0)); }

			while count >= 3 {
				let count2 = count.min(6);
				if out.is_some() { rle.push((16, count2 as u32 - 3)); }
				cl_counts[16] += 1;
				count -= count2;
			}
		}

		// Deal with non- or insufficiently-repeating values.
		cl_counts[symbol as usize] += count;
		if out.is_some() {
			for _ in 0..count { rle.push((symbol, 0)); }
		}

		i += 1;
	}

	// Update the lengths and symbols given the counts.
	zopfli_length_limited_code_lengths::<7, 19>(&cl_counts, &mut cl_lengths)?;

	// Find the last non-zero index of the counts table.
	// Safety: all ORDER values are between 0..19.
	let mut hclen = 15;
	while hclen > 0 && cl_counts[DEFLATE_ORDER[hclen + 3] as usize] == 0 {
		hclen -= 1;
	}

	// Write the tree!
	if let Some(out) = out {
		// Convert the lengths to symbols.
		let mut cl_symbols = [0_u32; 19];
		zopfli_lengths_to_symbols::<8, 19>(&cl_lengths, &mut cl_symbols);

		// Write the main lengths.
		out.add_bits(hlit as u32, 5);
		out.add_bits(hdist as u32, 5);
		out.add_bits(hclen as u32, 4);

		// Write each cl_length in the jumbled DEFLATE order.
		for &o in &DEFLATE_ORDER[..hclen + 4] {
			// Safety: all ORDER values are between 0..19.
			out.add_bits(cl_lengths[o as usize], 3);
		}

		// Write each symbol in order of appearance along with its extra bits,
		// if any.
		for (a, b) in rle {
			let symbol = cl_symbols[a as usize];
			out.add_huffman_bits(symbol, cl_lengths[a as usize]);

			// Extra bits.
			if a == 16 { out.add_bits(b, 2); }
			else if a == 17 { out.add_bits(b, 3); }
			else if a == 18 { out.add_bits(b, 7); }
		}
	}

	// Calculate the spatial requirements for all this data.
	let mut size = 14;         // hlit, hdist, hclen.
	size += (hclen + 4) * 3;   // cl_lengths.
	for (&a, b) in cl_lengths.iter().zip(cl_counts.iter()) {
		size += (a as usize) * b;
	}
	size += cl_counts[16] * 2; // Extra bits.
	size += cl_counts[17] * 3;
	size += cl_counts[18] * 7;

	Ok(size)
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
	store: &LZ77Store,
	mut start: usize,
	mut end: usize,
) -> Result<(usize, usize), ZopfliError> {
	// Keep track of the original start/end points.
	let split_start = start - 1;
	let split_end = end;

	let mut best_cost = usize::MAX;
	let mut best_idx = start;

	// Small chunks don't need much.
	if end - start < 1024 {
		for i in start..end {
			let cost = split_cost(store, split_start, i, split_end)?;
			if cost < best_cost {
				best_cost = cost;
				best_idx = i;
			}
		}
		return Ok((best_idx, best_cost));
	}

	// Divide and conquer.
	let mut p = [0_usize; MINIMUM_SPLIT_DISTANCE - 1];
	let mut last_best_cost = usize::MAX;
	while MINIMUM_SPLIT_DISTANCE <= end - start {
		let mut best_p_idx = 0;
		for (i, pp) in p.iter_mut().enumerate() {
			*pp = start + (i + 1) * ((end - start).wrapping_div(MINIMUM_SPLIT_DISTANCE));
			let line_cost =
				if best_idx == *pp { last_best_cost }
				else { split_cost(store, split_start, *pp, split_end)? };

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

	Ok((best_idx, last_best_cost))
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
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	ll_lengths: &mut [u32; ZOPFLI_NUM_LL],
	d_lengths: &mut [u32; ZOPFLI_NUM_D],
) -> Result<usize, ZopfliError> {
	// Populate some counts.
	let (mut ll_counts, d_counts) = store.histogram(lstart, lend)?;
	ll_counts[256] = 1;

	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_LL>(&ll_counts, ll_lengths)?;
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_D>(&d_counts, d_lengths)?;

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

/// # Symbol Span Range.
///
/// Convert an LZ77 range to the start/end positions of the block.
fn get_lz77_byte_range(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<(usize, usize), ZopfliError> {
	let slice = store.entries.as_slice();
	if lstart < lend && lend <= slice.len() {
		let instart = slice[lstart].pos;
		let e = slice[lend - 1];
		Ok((instart, e.length() as usize + e.pos))
	}
	else { Err(zopfli_error!()) }
}

/// # Optimal LZ77.
///
/// Calculate lit/len and dist pairs for the dataset.
///
/// Note: this incorporates the functionality of `ZopfliLZ77OptimalRun`
/// directly.
///
/// This is a rewrite of the original `squeeze.c` method.
fn lz77_optimal(
	arr: &[u8],
	instart: usize,
	numiterations: i32,
	store: &mut LZ77Store,
	scratch_store: &mut LZ77Store,
	state: &mut ZopfliState,
) -> Result<(), ZopfliError> {
	// Easy abort.
	if instart >= arr.len() { return Ok(()); }

	// Reset the main cache for the current blocksize.
	state.init_lmc(arr.len() - instart);

	// Greedy run.
	scratch_store.clear();
	state.greedy(arr, instart, scratch_store, Some(instart))?;

	// Create new stats with the store (updated by the greedy pass).
	let mut current_stats = SymbolStats::new();
	current_stats.load_store(scratch_store);

	// Set up dummy stats we can use to track best and last.
	let mut ran = RanState::new();
	let mut best_stats = SymbolStats::new();

	// We'll also want dummy best and last costs.
	let mut last_cost = 0;
	let mut best_cost = usize::MAX;

	// Repeat statistics with the cost model from the previous
	// stat run.
	let mut last_ran = -1;
	for i in 0..numiterations.max(0) {
		// Reset the LZ77 store.
		scratch_store.clear();

		// Optimal run.
		state.optimal_run(
			arr,
			instart,
			Some(&current_stats),
			scratch_store,
		)?;

		// This is the cost we actually care about.
		let current_cost = calculate_block_size(
			scratch_store,
			0,
			scratch_store.len(),
			BlockType::Dynamic,
		)?;

		// We have a new best!
		if current_cost < best_cost {
			store.replace(scratch_store);
			best_stats = current_stats;
			best_cost = current_cost;
		}

		// Copy the stats to last_stats, clear them, and repopulate
		// with the current store.
		let (last_litlens, last_dists) = current_stats.clear();
		current_stats.load_store(scratch_store);

		// Once the randomness has kicked in, improve convergence by
		// weighting the current and previous stats.
		if last_ran != -1 {
			current_stats.add_last(&last_litlens, &last_dists);
			current_stats.crunch();
		}

		// Replace the current stats with the best stats, randomize,
		// and see what happens.
		if 5 < i && current_cost == last_cost {
			current_stats = best_stats;
			current_stats.randomize(&mut ran);
			current_stats.crunch();
			last_ran = i;
		}

		last_cost = current_cost;
	}

	Ok(())
}

/// # Split Block Cost.
///
/// Return the sum of the estimated costs of the left and right sections of the
/// data.
fn split_cost(store: &LZ77Store, start: usize, mid: usize, end: usize) -> Result<usize, ZopfliError> {
	Ok(
		calculate_block_size_auto_type(store, start, mid)? +
		calculate_block_size_auto_type(store, mid, end)?
	)
}

#[allow(unsafe_code, clippy::integer_division)]
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
					// Safety: the compiler doesn't understand (i < four) means
					// we have at least i+3 entries left.
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

#[allow(clippy::too_many_arguments)]
#[inline(never)]
/// # (Maybe) Add LZ77 Expensive Fixed Block.
///
/// This runs the full suite of fixed-tree tests on the data and writes it to
/// the output if it is indeed better than the uncompressed/dynamic variants.
///
/// Returns `true` if data was written.
fn try_lz77_expensive_fixed(
	store: &LZ77Store,
	fixed_store: &mut LZ77Store,
	state: &mut ZopfliState,
	uncompressed_cost: usize,
	dynamic_cost: usize,
	arr: &[u8],
	lstart: usize,
	lend: usize,
	last_block: bool,
	expected_data_size: usize,
	out: &mut ZopfliOut,
) -> Result<bool, ZopfliError> {
	// Safety: the split points are checked at creation.
	let (instart, inend) = get_lz77_byte_range(store, lstart, lend)?;

	// Run all the expensive fixed-cost checks.
	state.init_lmc(inend - instart);

	// Pull the hasher.
	fixed_store.clear();
	state.optimal_run(
		arr.get(..inend).ok_or(zopfli_error!())?,
		instart,
		None,
		fixed_store,
	)?;

	// Find the resulting cost.
	let fixed_cost = calculate_block_size(
		fixed_store,
		0,
		fixed_store.len(),
		BlockType::Fixed,
	)?;

	// If it is better than dynamic, and uncompressed isn't better than both
	// fixed and dynamic, it's the best and worth writing!
	if fixed_cost < dynamic_cost && (fixed_cost <= uncompressed_cost || dynamic_cost <= uncompressed_cost) {
		add_lz77_block(
			BlockType::Fixed, last_block, fixed_store, arr, 0, fixed_store.len(),
			expected_data_size, out,
		)
			.map(|()| true)
	}
	else { Ok(false) }
}

/// # Try Huffman RLE Optimization.
///
/// This method attempts to optimize the RLE parts of the block, saving the
/// result if better, ignoring it if not.
///
/// The size of the encoded tree and data (in bits) is returned, minus the
/// 3-bit block header.
fn try_optimize_huffman_for_rle(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	ll_counts: &[usize; ZOPFLI_NUM_LL],
	d_counts: &[usize; ZOPFLI_NUM_D],
	ll_lengths: &mut [u32; ZOPFLI_NUM_LL],
	d_lengths: &mut [u32; ZOPFLI_NUM_D],
) -> Result<usize, ZopfliError> {
	// Calculate the tree and data sizes as are.
	let (_, treesize) = calculate_tree_size(ll_lengths, d_lengths)?;
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
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_LL>(&ll_counts2, &mut ll_lengths2)?;
	zopfli_length_limited_code_lengths::<15, ZOPFLI_NUM_D>(&d_counts2, &mut d_lengths2)?;
	patch_distance_codes(&mut d_lengths2);

	// Calculate the optimized tree and data sizes.
	let (_, treesize2) = calculate_tree_size(&ll_lengths2, &d_lengths2)?;
	let datasize2 = calculate_block_symbol_size_given_counts(
		ll_counts,
		d_counts,
		&ll_lengths2,
		&d_lengths2,
		store,
		lstart,
		lend,
	);

	// Return whichever's better.
	let sum = treesize + datasize;
	let sum2 = treesize2 + datasize2;
	if sum <= sum2 { Ok(sum) }
	else {
		ll_lengths.copy_from_slice(ll_lengths2.as_slice());
		d_lengths.copy_from_slice(d_lengths2.as_slice());
		Ok(sum2)
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
