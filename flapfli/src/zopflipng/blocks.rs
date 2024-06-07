/*!
# Flapfli: Blocks.

This module contains the deflate entrypoint and all of the block-related odds
and ends that didn't make it into other modules.
*/

use dactyl::NoHash;
use std::{
	cell::Cell,
	collections::HashSet,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	DISTANCE_BITS,
	DISTANCE_VALUES,
	FIXED_SYMBOLS_D,
	FIXED_SYMBOLS_LL,
	FIXED_TREE_D,
	FIXED_TREE_LL,
	LENGTH_SYMBOLS_BITS_VALUES,
	LengthLimitedCodeLengths,
	LZ77Store,
	stats::{
		RanState,
		SymbolStats,
	},
	TreeLd,
	zopfli_error,
	ZopfliError,
	ZopfliOut,
	ZopfliState,
};



/// # Length Symbol Extra Bits.
const LENGTH_EXTRA_BITS: [u32; 29] = [
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

			Err(zopfli_error!())
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
		store2.clear();

		// Now some LZ77 funny business.
		let mut cost1 = 0;
		let mut store3 = LZ77Store::new();
		for i in 0..=limit {
			let start = if i == 0 { instart } else { self.slice1[i - 1] };
			let end = if i < limit { self.slice1[i] } else { arr.len() };

			// This assertion is redundant as we explicitly check range sanity
			// earlier and later in the pipeline.
			debug_assert!(start <= end && end <= arr.len());

			// Make another store.
			lz77_optimal(
				arr.get(..end).ok_or(zopfli_error!())?,
				start,
				numiterations,
				store2,
				&mut store3,
				state,
			)?;
			cost1 += calculate_block_size_auto_type(store2, 0, store2.len())?;

			// Append its data to our main store.
			store.steal_entries(store2);

			// Save the chunk size to our best.
			if i < limit { self.slice2[i] = store.len(); }
		}
		drop(store3);

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
	let mut store = LZ77Store::new();
	let mut store2 = LZ77Store::new();

	// Find the split points.
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
	counts: &'a [Cell<u32>],
	good: usize,
	bad: usize,
}

impl<'a> GoodForRle<'a> {
	/// # New Instance.
	const fn new(counts: &'a [Cell<u32>]) -> Self {
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
		let scratch = self.counts[0].get();
		let mut stride = 0;
		while let [count, rest @ ..] = self.counts {
			// Note the reptition and circle back around. This will always
			// trigger on the first pass, so stride will always be at least
			// one.
			if count.get() == scratch {
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
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Uncompressed blocks are easy!
	if matches!(btype, BlockType::Uncompressed) {
		let (instart, inend) = store.byte_range(lstart, lend)?;
		out.add_uncompressed_block(last_block, arr, instart, inend);
		return Ok(());
	}

	// Add some bits.
	out.add_bit(u8::from(last_block));
	out.add_bit((btype as u8) & 1);
	out.add_bit(((btype as u8) & 2) >> 1);

	// Write the rest according to the block type!
	if matches!(btype, BlockType::Fixed) {
		add_lz77_block_fixed(store, lstart, lend, out)
	}
	else {
		add_lz77_block_dynamic(store, lstart, lend, out)
	}
}

#[inline(never)]
/// # Add LZ77 Block (Dynamic).
///
/// This finishes the work started by `add_lz77_block`.
fn add_lz77_block_dynamic(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Build the lengths first.
	let (extra, _, ll_lengths, d_lengths) = get_dynamic_lengths(
		store,
		lstart,
		lend,
	)?;
	TreeLd::encode_tree(&ll_lengths, &d_lengths, extra, out)?;

	// Now we need the symbols.
	let ll_symbols = ArrayLL::<u32>::llcl_symbols(&ll_lengths)?;
	let d_symbols = ArrayD::<u32>::llcl_symbols(&d_lengths)?;

	// Write all the data!
	add_lz77_data(
		store, lstart, lend, &ll_symbols, &ll_lengths, &d_symbols, &d_lengths, out
	)?;

	// Finish up by writting the end symbol.
	out.add_huffman_bits(ll_symbols[256], ll_lengths[256] as u32);
	Ok(())
}

/// # Add LZ77 Block (Fixed).
///
/// This finishes the work started by `add_lz77_block`.
fn add_lz77_block_fixed(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Write all the data!
	add_lz77_data(
		store, lstart, lend,
		&FIXED_SYMBOLS_LL, &FIXED_TREE_LL, &FIXED_SYMBOLS_D, &FIXED_TREE_D,
		out
	)?;

	// Finish up by writting the end symbol.
	out.add_huffman_bits(FIXED_SYMBOLS_LL[256], FIXED_TREE_LL[256] as u32);
	Ok(())
}

#[allow(
	clippy::cast_precision_loss,
	clippy::cast_sign_loss,
	clippy::too_many_arguments,
)]
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
	let uncompressed_cost = calculate_block_size_uncompressed(store, lstart, lend)?;
	let fixed_cost = calculate_block_size_fixed(store, lstart, lend);
	let dynamic_cost = calculate_block_size_dynamic(store, lstart, lend)?;

	// Fixed stores are only useful up to a point; we can skip the overhead
	// if the store is big or the dynamic cost estimate is unimpressive.
	if
		(store.len() < 1000 || fixed_cost * 10 <= dynamic_cost * 11) &&
		try_lz77_expensive_fixed(
			store, fixed_store, state, uncompressed_cost, dynamic_cost,
			arr, lstart, lend, last_block,
			out,
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
	add_lz77_block(btype, last_block, store, arr, lstart, lend, out)
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
	ll_symbols: &ArrayLL<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_symbols: &ArrayD<u32>,
	d_lengths: &ArrayD<DeflateSym>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	for e in store.entries.get(lstart..lend).ok_or(zopfli_error!())? {
		// Length only.
		if e.dist <= 0 {
			if (e.litlen as u16) >= 256 {
				return Err(zopfli_error!());
			}
			if ll_lengths[e.litlen as usize].is_zero() { return Err(zopfli_error!()); }

			out.add_huffman_bits(
				ll_symbols[e.litlen as usize],
				ll_lengths[e.litlen as usize] as u32,
			);
		}
		// Length and distance.
		else {
			let (symbol, bits, value) = LENGTH_SYMBOLS_BITS_VALUES[e.litlen as usize];
			if ll_lengths[symbol as usize].is_zero() { return Err(zopfli_error!()); }

			out.add_huffman_bits(
				ll_symbols[symbol as usize],
				ll_lengths[symbol as usize] as u32,
			);
			out.add_bits(u32::from(value), bits);

			// Now the distance bits.
			if d_lengths[e.d_symbol as usize].is_zero() { return Err(zopfli_error!()); }
			out.add_huffman_bits(
				d_symbols[e.d_symbol as usize],
				d_lengths[e.d_symbol as usize] as u32,
			);
			out.add_bits(
				u32::from(DISTANCE_VALUES[e.dist as usize]),
				u32::from(DISTANCE_BITS[e.d_symbol as usize]),
			);
		}
	}

	Ok(())
}

#[allow(clippy::cast_possible_truncation)] // The maximum blocksize is only 1 million.
/// # Calculate Block Size (Uncompressed).
fn calculate_block_size_uncompressed(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<u32, ZopfliError> {
	let (instart, inend) = store.byte_range(lstart, lend)?;
	let blocksize = (inend - instart) as u32;

	// Blocks larger than u16::MAX need to be split.
	let blocks = blocksize.div_ceil(65_535);
	Ok(blocks * 40 + blocksize * 8)
}

/// # Calculate Block Size (Fixed).
fn calculate_block_size_fixed(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> u32 {
	// The end symbol is always included.
	let mut size = FIXED_TREE_LL[256] as u32;

	// Loop the store if we have data to loop.
	let slice = store.entries.as_slice();
	if lstart < lend && lend <= slice.len() {
		// Make sure the end does not exceed the store!
		for e in &slice[lstart..lend] {
			if e.dist <= 0 {
				size += FIXED_TREE_LL[e.litlen as usize] as u32;
			}
			else {
				size += LENGTH_SYMBOLS_BITS_VALUES[e.litlen as usize].1;
				size += FIXED_TREE_LL[e.ll_symbol as usize] as u32;
				size += u32::from(DISTANCE_BITS[e.d_symbol as usize]);
				size += FIXED_TREE_D[e.d_symbol as usize] as u32;
			}
		}
	}

	size
}

#[inline(never)]
/// # Calculate Block Size (Dynamic).
fn calculate_block_size_dynamic(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<u32, ZopfliError> {
	get_dynamic_lengths(store, lstart, lend).map(|(_, size, _, _)| size)
}

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto_type(
	store: &LZ77Store,
	lstart: usize,
	lend: usize,
) -> Result<u32, ZopfliError> {
	let uncompressed_cost = calculate_block_size_uncompressed(store, lstart, lend)?;

	// We can skip the expensive fixed-cost calculations for large blocks since
	// they're unlikely ever to use it.
	let fixed_cost =
		if 1000 < store.len() { uncompressed_cost }
		else { calculate_block_size_fixed(store, lstart, lend) };

	let dynamic_cost = calculate_block_size_dynamic(store, lstart, lend)?;

	// If uncompressed is better than everything, return it.
	if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost {
		Ok(uncompressed_cost)
	}
	// Otherwise choose the smaller of fixed and dynamic.
	else if fixed_cost < dynamic_cost { Ok(fixed_cost) }
	else { Ok(dynamic_cost) }
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
) -> Result<(usize, u32), ZopfliError> {
	// Keep track of the original start/end points.
	let split_start = start - 1;
	let split_end = end;

	let mut best_cost = u32::MAX;
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
	let mut last_best_cost = u32::MAX;
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

/// # Try Huffman RLE Optimization.
///
/// This method attempts to optimize the RLE parts of the block, saving the
/// result if better, ignoring it if not.
///
/// The size of the encoded tree and data (in bits) is returned, minus the
/// 3-bit block header.
fn get_dynamic_lengths(store: &LZ77Store, lstart: usize, lend: usize)
-> Result<(u8, u32, ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
	#[allow(unsafe_code)]
	/// # Revisualize as Bytes.
	///
	/// The compiler might be able to optimize `[u8]` comparisons better than
	/// `[DeflateSym]` ones, even though they're equivalent.
	const fn bytes<const N: usize>(arr: &[DeflateSym; N]) -> &[u8; N] {
		// Safety: DeflateSym has the same size and alignment as u8.
		unsafe { &* arr.as_ptr().cast() }
	}

	/// # Calculate Dynamic Block Size.
	fn data_size(
		ll_counts: &ArrayLL<u32>,
		d_counts: &ArrayD<u32>,
		ll_lengths: &ArrayLL<DeflateSym>,
		d_lengths: &ArrayD<DeflateSym>,
	) -> u32 {
		// The end symbol is always included.
		let mut result = ll_lengths[256] as u32;

		// The early lengths and counts.
		for (ll, lc) in ll_lengths.iter().copied().zip(ll_counts).take(256) {
			result += (ll as u32) * lc;
		}

		// The lengths and counts with extra bits.
		for (i, lbit) in (257..257 + LENGTH_EXTRA_BITS.len()).zip(LENGTH_EXTRA_BITS) {
			result += (ll_lengths[i] as u32 + lbit) * ll_counts[i];
		}

		// The distance lengths, counts, and extra bits.
		for (i, dbit) in DISTANCE_BITS.iter().copied().enumerate().take(30) {
			result += (d_lengths[i] as u32 + u32::from(dbit)) * d_counts[i];
		}

		result
	}

	/// # Dynamic Length-Limited Code Lengths.
	///
	/// Calculate, patch, and return the distance code length symbols.
	fn d_llcl(d_counts: &ArrayD<u32>)
	-> Result<ArrayD<DeflateSym>, ZopfliError> {
		let mut d_lengths = d_counts.llcl()?;

		// Buggy decoders require at least two non-zero distances. Let's ese
		// what we've got!
		let mut one: Option<bool> = None;
		for (i, dist) in d_lengths.iter().copied().enumerate().take(30) {
			// We have (at least) two non-zero entries; no patching needed!
			if ! dist.is_zero() && one.replace(i == 0).is_some() { return Ok(d_lengths); }
		}

		match one {
			// The first entry had a code, so patching the second gives us two.
			Some(true) => { d_lengths[1] = DeflateSym::D01; },
			// The first entry didn't have a code, so patching it gives us two.
			Some(false) => { d_lengths[0] = DeflateSym::D01; },
			// There were no codes, so we can just patch the first two.
			None => {
				d_lengths[0] = DeflateSym::D01;
				d_lengths[1] = DeflateSym::D01;
			},
		}

		Ok(d_lengths)
	}

	#[inline(never)]
	fn optimized_counts<const N: usize>(counts: &[u32; N]) -> [u32; N] {
		let mut counts2 = *counts;
		optimize_huffman_for_rle(&mut counts2);
		counts2
	}

	// Pull the counts from the store.
	let (mut ll_counts, d_counts) = store.histogram(lstart, lend);
	ll_counts[256] = 1;

	// Get the length-limited symbols.
	let ll_lengths = ll_counts.llcl()?;
	let d_lengths = d_llcl(&d_counts)?;

	// Calculate the tree and sizes.
	let (extra, treesize) = TreeLd::calculate_tree_size(&ll_lengths, &d_lengths)?;
	let datasize = data_size(&ll_counts, &d_counts, &ll_lengths, &d_lengths);
	let sum = treesize + datasize;

	// Now copy and optimize the counts, then redo the LLCL. (Note: we only
	// need to keep the latter.)
	let ll_counts2 = optimized_counts(&ll_counts);
	let d_counts2 = optimized_counts(&d_counts);
	let ll_lengths2 = ll_counts2.llcl()?;
	let d_lengths2 = d_llcl(&d_counts2)?;

	// Assuming we got different symbols, let's find the optimized sizes.
	if
		bytes(&d_lengths) != bytes(&d_lengths2) ||
		bytes(&ll_lengths) != bytes(&ll_lengths2)
	{
		let (extra2, treesize2) = TreeLd::calculate_tree_size(&ll_lengths2, &d_lengths2)?;

		// Note: this really does require the *original* counts.
		let datasize2 = data_size(&ll_counts, &d_counts, &ll_lengths2, &d_lengths2);
		let sum2 = treesize2 + datasize2;

		// Return if better!
		if sum2 < sum {
			return Ok((extra2, sum2, ll_lengths2, d_lengths2));
		}
	}

	// It was fine as it was!
	Ok((extra, sum, ll_lengths, d_lengths))
}

/// # Optimal LZ77.
///
/// Calculate lit/len and dist pairs for the dataset.
///
/// Note: this incorporates the functionality of `ZopfliLZ77OptimalRun`
/// directly.
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
	let mut best_cost = u32::MAX;

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
		let current_cost = calculate_block_size_dynamic(
			scratch_store,
			0,
			scratch_store.len(),
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

#[allow(clippy::inline_always, clippy::integer_division)]
#[inline(always)]
/// # Optimize Huffman RLE Compression.
///
/// Change the population counts to improve Huffman tree compression,
/// particularly its RLE part.
fn optimize_huffman_for_rle(mut counts: &mut [u32]) {
	// Convert counts to a proper slice with trailing zeroes trimmed.
	while let [ rest @ .., 0 ] = counts { counts = rest; }
	if counts.is_empty() { return; }

	// We need to read and write simultaneously; once again the Cell trick can
	// keep us safe!
	let counts = Cell::from_mut(counts).as_slice_of_cells();

	// Find collapseable ranges!
	let mut stride: u32 = 0;
	let mut scratch: u32 = counts[0].get();
	let mut sum: u32 = 0;
	for (i, (count, good)) in counts.iter().map(Cell::get).zip(GoodForRle::new(counts)).enumerate() {
		// Time to reset (and maybe collapse).
		if good || count.abs_diff(scratch) >= 4 {
			// Collapse the stride if it is as least four and contained
			// something non-zero.
			if sum != 0 && stride >= 4 {
				let v = ((sum + stride / 2) / stride).max(1);
				// This condition just helps the compiler understand the range
				// won't overflow; it can't, but it doesn't know that.
				if let Some(from) = i.checked_sub(stride as usize) {
					for c in &counts[from..i] { c.set(v); }
				}
			}

			// Reset!
			stride = 0;
			sum = 0;

			// If there are at least three future counts, we can set scratch
			// to a sorted weighted average, otherwise the current value will
			// do.
			scratch = counts.get(i..i + 4).map_or(
				count,
				|c| c.iter().fold(2, |a, c| a + c.get()) / 4
			);
		}

		stride += 1;
		sum += count;
	}

	// Collapse the trailing stride, if any.
	if sum != 0 && stride >= 4 {
		let v = ((sum + stride / 2) / stride).max(1);
		// This condition just helps the compiler understand the range won't
		// overflow; it can't, but it doesn't know that.
		if let Some(from) = counts.len().checked_sub(stride as usize) {
			for c in &counts[from..] { c.set(v); }
		}
	}
}

/// # Split Block Cost.
///
/// Return the sum of the estimated costs of the left and right sections of the
/// data.
fn split_cost(store: &LZ77Store, start: usize, mid: usize, end: usize) -> Result<u32, ZopfliError> {
	let a = calculate_block_size_auto_type(store, start, mid)?;
	let b = calculate_block_size_auto_type(store, mid, end)?;
	Ok(a + b)
}

#[allow(clippy::too_many_arguments)]
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
	uncompressed_cost: u32,
	dynamic_cost: u32,
	arr: &[u8],
	lstart: usize,
	lend: usize,
	last_block: bool,
	out: &mut ZopfliOut,
) -> Result<bool, ZopfliError> {
	let (instart, inend) = store.byte_range(lstart, lend)?;

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
	let fixed_cost = calculate_block_size_fixed(
		fixed_store,
		0,
		fixed_store.len(),
	);

	// If it is better than dynamic, and uncompressed isn't better than both
	// fixed and dynamic, it's the best and worth writing!
	if fixed_cost < dynamic_cost && (fixed_cost <= uncompressed_cost || dynamic_cost <= uncompressed_cost) {
		add_lz77_block(BlockType::Fixed, last_block, fixed_store, arr, 0, fixed_store.len(), out)
			.map(|()| true)
	}
	else { Ok(false) }
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_fixed_symbols() {
		assert_eq!(
			ArrayLL::<u32>::llcl_symbols(&FIXED_TREE_LL),
			Ok(FIXED_SYMBOLS_LL),
		);
		assert_eq!(
			ArrayD::<u32>::llcl_symbols(&FIXED_TREE_D),
			Ok(FIXED_SYMBOLS_D),
		);
	}

	#[test]
	fn t_good_for_rle() {
		for c in [
			[196, 23, 10, 12, 5, 4, 1, 23, 8, 2, 6, 5, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 47, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 8, 1, 1, 1, 0, 4, 7, 2, 0, 1, 10, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 42, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 44, 284, 1, 15, 21, 0, 55, 0, 19, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 320, 12, 0, 0, 17, 3, 0, 3, 2].as_mut_slice(),
			[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 0, 288, 11, 41, 6, 5, 2, 0, 0, 0, 1].as_mut_slice(),
			[201, 24, 10, 12, 5, 4, 1, 24, 8, 2, 6, 4, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 49, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 9, 1, 1, 1, 0, 4, 6, 2, 0, 1, 8, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 44, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 46, 281, 1, 20, 3, 10, 59, 0, 4, 12, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 318, 12, 0, 0, 21, 0, 0, 3, 2].as_mut_slice(),
		] {
			let c = Cell::from_mut(c).as_slice_of_cells();

			// Make sure our ExactSizeness is working.
			let good = GoodForRle::new(c);
			assert_eq!(
				good.len(),
				c.len(),
				"GoodForRle iterator count does not match source.",
			);

			// And make sure we actually collect that count!
			let good = good.collect::<Vec<bool>>();
			assert_eq!(
				good.len(),
				c.len(),
				"Collected GoodForRle iterator count does not match source.",
			);
		}
	}
}
