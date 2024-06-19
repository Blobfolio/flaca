/*!
# Flapfli: Blocks.

This module contains the deflate entrypoint and all of the block-related odds
and ends that didn't make it into other modules.
*/

use dactyl::NoHash;
use std::{
	collections::HashSet,
	num::NonZeroU32,
	ops::Range,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	DISTANCE_BITS,
	DISTANCE_VALUES,
	encode_tree,
	FIXED_SYMBOLS_D,
	FIXED_SYMBOLS_LL,
	FIXED_TREE_D,
	FIXED_TREE_LL,
	get_dynamic_lengths,
	LENGTH_SYMBOL_BIT_VALUES,
	LENGTH_SYMBOL_BITS,
	LengthLimitedCodeLengths,
	LZ77Store,
	SplitLen,
	SplitPIdx,
	SymbolIteration,
	stats::{
		RanState,
		SymbolStats,
	},
	zopfli_error,
	ZopfliError,
	ZopfliOut,
	ZopfliState,
};



const BLOCK_TYPE_FIXED: u8 = 1;
const BLOCK_TYPE_DYNAMIC: u8 = 2;

/// # Minimum Split Distance.
const MINIMUM_SPLIT_DISTANCE: usize = 10;

#[allow(unsafe_code)]
const NZ10: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(10) };
#[allow(unsafe_code)]
const NZ11: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(11) };



/// # Split Point Scratch.
///
/// This holds two sets of block split points for use during the deflate
/// passes. Each set can hold up to 14 points (one less than
/// `BLOCKSPLITTING_MAX`), but we're overallocating to 15 to cheaply elide
/// bounds checks.
///
/// A single instance of this struct is (re)used for all deflate passes on a
/// given image to reduce allocation overhead.
pub(crate) struct SplitPoints {
	slice1: [usize; 15],
	slice2: [usize; 15],
	done: HashSet<usize, NoHash>,
}

impl SplitPoints {
	/// # New Instance.
	pub(crate) fn new() -> Self {
		Self {
			slice1: [0; 15],
			slice2: [0; 15],
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
	-> Result<SplitLen, ZopfliError> {
		// Populate an LZ77 store from a greedy pass. This results in better
		// block choices than a full optimal pass.
		state.greedy_cold(arr, instart, store, None)?;

		// Do an LZ77 pass.
		let len = self.split_lz77(store)?;

		// Find the corresponding uncompressed positions.
		if len.is_zero() { Ok(len) }
		else {
			let mut pos = instart;
			let mut j = SplitLen::S00;
			for (i, e) in store.entries.iter().enumerate().take(self.slice2[len as usize - 1] + 1) {
				if i == self.slice2[j as usize] {
					self.slice1[j as usize] = pos;
					j = j.increment();
					if (j as u8) == (len as u8) { return Ok(len); }
				}
				pos += e.length() as usize;
			}

			Err(zopfli_error!())
		}
	}

	/// # LZ77 Split Pass.
	///
	/// This sets the LZ77 split points according to convoluted cost
	/// evaluations.
	fn split_lz77(&mut self, store: &LZ77Store) -> Result<SplitLen, ZopfliError> {
		/// # Find Largest Splittable Block.
		///
		/// This finds the largest available block for splitting, evenly spreading the
		/// load if a limited number of blocks are requested.
		///
		/// Returns `false` if no blocks are found.
		fn find_largest(
			lz77size: usize,
			done: &HashSet<usize, NoHash>,
			splitpoints: &[usize],
			rng: &mut Range<usize>,
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
					rng.start = start;
					rng.end = end;
					best = end - start;
				}
			}
			MINIMUM_SPLIT_DISTANCE <= best
		}

		// This won't work on tiny files.
		if store.len() < MINIMUM_SPLIT_DISTANCE { return Ok(SplitLen::S00); }

		// Get started!
		self.done.clear();
		let mut rng = 0..store.len();
		let mut last = 0;
		let mut len = SplitLen::S00;
		loop {
			let (llpos, llcost) = find_minimum_cost(store, rng.start + 1..rng.end)?;
			if llpos <= rng.start || llpos >= rng.end {
				return Err(zopfli_error!());
			}

			// Ignore points we've already covered.
			if llpos == rng.start + 1 || calculate_block_size_auto_type(store, rng.clone())? < llcost {
				self.done.insert(rng.start);
			}
			else {
				// Mark it as a split point and add it sorted.
				self.slice2[len as usize] = llpos;
				len = len.increment();

				// Keep the list sorted.
				if last > llpos { self.slice2[..len as usize].sort_unstable(); }
				else { last = llpos; }

				// Stop if we've split the maximum number of times.
				if len.is_max() { break; }
			}

			// Look for a split and adjust the start/end accordingly. If we don't
			// find one or the remaining distance is too small to continue, we're
			// done!
			if ! find_largest(
				store.len(),
				&self.done,
				&self.slice2[..len as usize],
				&mut rng,
			) { break; }
		}

		Ok(len)
	}

	/// # (Re)split Best.
	///
	/// If there's enough data, resplit with optimized LZ77 paths and return
	/// whichever best is better.
	fn split_again(
		&mut self,
		store: &LZ77Store,
		limit1: SplitLen,
		cost1: u32,
	) -> Result<&[usize], ZopfliError> {
		if 1 < (limit1 as u8) {
			// Move slice2 over to slice1 so we can repopulate slice2.
			self.slice1.copy_from_slice(self.slice2.as_slice());

			let limit2 = self.split_lz77(store)?;
			let mut cost2 = 0;
			for i in 0..=limit2 as usize {
				let start = if i == 0 { 0 } else { self.slice2[i - 1] };
				let end = if i < (limit2 as usize) { self.slice2[i] } else { store.len() };
				cost2 += calculate_block_size_auto_type(store, start..end)?.get();
			}

			// It's better!
			if cost2 < cost1 { Ok(&self.slice2[..limit2 as usize]) }
			else { Ok(&self.slice1[..limit1 as usize]) }
		}
		else { Ok(&self.slice2[..limit1 as usize]) }
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
		let limit = self.split_raw(arr, instart, state, store2)?;
		store2.clear();

		// Now some LZ77 funny business.
		let mut cost1 = 0;
		let mut store3 = LZ77Store::new();
		for i in 0..=limit as usize {
			let start = if i == 0 { instart } else { self.slice1[i - 1] };
			let end = if i < (limit as usize) { self.slice1[i] } else { arr.len() };

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
			cost1 += calculate_block_size_auto_type(store2, 0..store2.len())?.get();

			// Append its data to our main store.
			store.steal_entries(store2);

			// Save the chunk size to our best.
			if i < (limit as usize) { self.slice2[i] = store.len(); }
		}

		// Try a second pass, recalculating the LZ77 splits with the updated
		// store details.
		self.split_again(store, limit, cost1)
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
		add_lz77_block(
			last_block && i == best.len(),
			&store,
			&mut store2,
			state,
			arr,
			start..end,
			out,
		)?;
	}

	Ok(())
}



#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
/// # Add LZ77 Block (Automatic Type).
///
/// This calculates the expected output sizes for all three block types, then
/// writes the best one to the output file.
fn add_lz77_block(
	last_block: bool,
	store: &LZ77Store,
	fixed_store: &mut LZ77Store,
	state: &mut ZopfliState,
	arr: &[u8],
	rng: Range<usize>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	/// # Add LZ77 Block (Dynamic).
	fn add_dynamic(
		last_block: bool,
		store: &LZ77Store,
		rng: Range<usize>,
		out: &mut ZopfliOut,
		extra: u8,
		ll_lengths: &ArrayLL<DeflateSym>,
		d_lengths: &ArrayD<DeflateSym>,
	) -> Result<(), ZopfliError> {
		// Type Bits.
		out.add_bit(u8::from(last_block));
		out.add_bit(BLOCK_TYPE_DYNAMIC & 1);
		out.add_bit((BLOCK_TYPE_DYNAMIC & 2) >> 1);

		// Build the lengths first.
		encode_tree(ll_lengths, d_lengths, extra, out)?;

		// Now we need the symbols.
		let ll_symbols = ArrayLL::<u32>::llcl_symbols(ll_lengths);
		let d_symbols = ArrayD::<u32>::llcl_symbols(d_lengths);

		// Write all the data!
		add_lz77_data(
			store, rng, &ll_symbols, ll_lengths, &d_symbols, d_lengths, out
		)?;

		// Finish up by writting the end symbol.
		out.add_huffman_bits(ll_symbols[256], ll_lengths[256] as u32);
		Ok(())
	}

	/// # Add LZ77 Block (Fixed).
	fn add_fixed(
		last_block: bool,
		store: &LZ77Store,
		rng: Range<usize>,
		out: &mut ZopfliOut,
	) -> Result<(), ZopfliError> {
		// Type Bits.
		out.add_bit(u8::from(last_block));
		out.add_bit(BLOCK_TYPE_FIXED & 1);
		out.add_bit((BLOCK_TYPE_FIXED & 2) >> 1);

		// Write all the data!
		add_lz77_data(
			store, rng,
			&FIXED_SYMBOLS_LL, &FIXED_TREE_LL, &FIXED_SYMBOLS_D, &FIXED_TREE_D,
			out
		)?;

		// Finish up by writting the end symbol.
		out.add_huffman_bits(FIXED_SYMBOLS_LL[256], FIXED_TREE_LL[256] as u32);
		Ok(())
	}

	#[inline(never)]
	fn dynamic_details(store: &LZ77Store, rng: Range<usize>)
	-> Result<(u8, NonZeroU32, ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
		get_dynamic_lengths(store, rng)
	}

	#[inline(never)]
	fn fixed_cost_cold(store: &LZ77Store, rng: Range<usize>) -> NonZeroU32 {
		calculate_block_size_fixed(store, rng)
	}

	// If the block is empty, we can assume a fixed-tree layout.
	if rng.is_empty() {
		out.add_bits(u32::from(last_block), 1);
		out.add_bits(1, 2);
		out.add_bits(0, 7);
		return Ok(());
	}

	// Calculate the three costs.
	let uncompressed_cost = calculate_block_size_uncompressed(store, rng.clone())?;
	let (dynamic_extra, dynamic_cost, dynamic_ll, dynamic_d) = dynamic_details(store, rng.clone())?;

	// Most blocks won't benefit from a fixed tree layout, but if we've got a
	// tiny one or the unoptimized-fixed size is within 10% of the dynamic size
	// we should check it out.
	if
		store.len() <= 1000 ||
		calculate_block_size_fixed(store, rng.clone()).saturating_mul(NZ10) <= dynamic_cost.saturating_mul(NZ11)
	{
		let rng2 = store.byte_range(rng.clone())?;
		state.init_lmc(rng2.len());

		// Perform an optimal run.
		state.optimal_run_fixed(
			arr.get(..rng2.end).ok_or(zopfli_error!())?,
			rng2.start,
			fixed_store,
		)?;

		// And finally, the cost!
		let fixed_cost = fixed_cost_cold(fixed_store, 0..fixed_store.len());
		if fixed_cost < dynamic_cost && fixed_cost <= uncompressed_cost {
			return add_fixed(last_block, fixed_store, 0..fixed_store.len(), out);
		}
	}

	// Dynamic is best!
	if dynamic_cost <= uncompressed_cost {
		add_dynamic(
			last_block, store, rng, out,
			dynamic_extra, &dynamic_ll, &dynamic_d,
		)
	}
	// All the work we did earlier was fruitless; the block works best in an
	// uncompressed form.
	else {
		let rng = store.byte_range(rng)?;
		out.add_uncompressed_block(last_block, arr, rng);
		Ok(())
	}
}

#[allow(clippy::cast_sign_loss)]
/// # Add LZ77 Data.
///
/// This adds all lit/len/dist codes from the lists as huffman symbols, but not
/// the end code (256).
fn add_lz77_data(
	store: &LZ77Store,
	rng: Range<usize>,
	ll_symbols: &ArrayLL<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_symbols: &ArrayD<u32>,
	d_lengths: &ArrayD<DeflateSym>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	for e in store.entries.get(rng).ok_or(zopfli_error!())? {
		// Always add the length symbol (or literal).
		if ll_lengths[e.ll_symbol as usize].is_zero() { return Err(zopfli_error!()); }
		out.add_huffman_bits(
			ll_symbols[e.ll_symbol as usize],
			ll_lengths[e.ll_symbol as usize] as u32,
		);

		// Add the length symbol bits and distance stuff.
		if 0 < e.dist {
			out.add_bits(
				u32::from(LENGTH_SYMBOL_BIT_VALUES[e.litlen as usize]),
				u32::from(LENGTH_SYMBOL_BITS[e.litlen as usize]),
			);

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
		// If the distance is zero, the litlen must be a literal.
		else if (e.litlen as u16) >= 256 { return Err(zopfli_error!()); }
	}

	Ok(())
}

#[allow(clippy::cast_possible_truncation)] // The maximum blocksize is only 1 million.
/// # Calculate Block Size (Uncompressed).
fn calculate_block_size_uncompressed(store: &LZ77Store, rng: Range<usize>)
-> Result<NonZeroU32, ZopfliError> {
	let rng = store.byte_range(rng)?;
	let blocksize = rng.len() as u32;

	// Blocks larger than u16::MAX need to be split.
	let blocks = blocksize.div_ceil(65_535);
	NonZeroU32::new(blocks * 40 + blocksize * 8).ok_or(zopfli_error!())
}

/// # Calculate Block Size (Fixed).
fn calculate_block_size_fixed(store: &LZ77Store, rng: Range<usize>) -> NonZeroU32 {
	// The end symbol is always included.
	let mut size = FIXED_TREE_LL[256] as u32;

	// Loop the store if we have data to loop.
	let slice = store.entries.as_slice();
	if rng.start < rng.end && rng.end <= slice.len() {
		// Make sure the end does not exceed the store!
		for e in &slice[rng] {
			size += FIXED_TREE_LL[e.ll_symbol as usize] as u32;
			if 0 < e.dist {
				size += u32::from(LENGTH_SYMBOL_BITS[e.litlen as usize]);
				size += u32::from(DISTANCE_BITS[e.d_symbol as usize]);
				size += FIXED_TREE_D[e.d_symbol as usize] as u32;
			}
		}
	}

	// This can't really fail, but fixed models are bullshit anyway so we can
	// fall back to an unbeatably large number.
	NonZeroU32::new(size).unwrap_or(NonZeroU32::MAX)
}

/// # Calculate Block Size (Dynamic).
fn calculate_block_size_dynamic(store: &LZ77Store, rng: Range<usize>)
-> Result<NonZeroU32, ZopfliError> {
	get_dynamic_lengths(store, rng).map(|(_, size, _, _)| size)
}

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto_type(store: &LZ77Store, rng: Range<usize>)
-> Result<NonZeroU32, ZopfliError> {
	let uncompressed_cost = calculate_block_size_uncompressed(store, rng.clone())?;

	// We can skip the expensive fixed-cost calculations for large blocks since
	// they're unlikely ever to use it.
	let fixed_cost =
		if 1000 < store.len() { uncompressed_cost }
		else { calculate_block_size_fixed(store, rng.clone()) };

	let dynamic_cost = calculate_block_size_dynamic(store, rng)?;

	// If uncompressed is better than everything, return it.
	if uncompressed_cost < fixed_cost && uncompressed_cost < dynamic_cost {
		Ok(uncompressed_cost)
	}
	// Otherwise choose the smaller of fixed and dynamic.
	else if fixed_cost < dynamic_cost { Ok(fixed_cost) }
	else { Ok(dynamic_cost) }
}

/// # Minimum Split Cost.
///
/// Return the index of the smallest split cost between `start..end`.
fn find_minimum_cost(store: &LZ77Store, mut rng: Range<usize>)
-> Result<(usize, NonZeroU32), ZopfliError> {
	/// # Split Block Cost.
	///
	/// Sum the left and right halves of the range.
	fn split_cost(store: &LZ77Store, start: usize, mid: usize, end: usize) -> Result<NonZeroU32, ZopfliError> {
		let a = calculate_block_size_auto_type(store, start..mid)?;
		let b = calculate_block_size_auto_type(store, mid..end)?;
		Ok(a.saturating_add(b.get()))
	}

	// Keep track of the original start/end points.
	let split_start = rng.start - 1;
	let split_end = rng.end;

	let mut best_cost = NonZeroU32::MAX;
	let mut best_idx = rng.start;

	// Small chunks don't need much.
	if rng.len() < 1024 {
		for i in rng {
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
	let mut last_best_cost = NonZeroU32::MAX;
	while MINIMUM_SPLIT_DISTANCE <= rng.len() {
		let mut best_p_idx = SplitPIdx::S0;
		for (i, pp) in SplitPIdx::all().zip(p.iter_mut()) {
			*pp = rng.start + (i as usize + 1) * (rng.len().wrapping_div(MINIMUM_SPLIT_DISTANCE));
			let line_cost =
				if best_idx == *pp { last_best_cost }
				else { split_cost(store, split_start, *pp, split_end)? };

			if (i as usize) == 0 || line_cost < best_cost {
				best_cost = line_cost;
				best_p_idx = i;
			}
		}

		// No improvement; we're done.
		if last_best_cost < best_cost { break; }

		// Nudge the boundaries and back again.
		best_idx = p[best_p_idx as usize];
		if 0 != (best_p_idx as usize) { rng.start = p[best_p_idx as usize - 1]; }
		if (best_p_idx as usize) + 1 < p.len() { rng.end = p[best_p_idx as usize + 1]; }

		last_best_cost = best_cost;
	}

	Ok((best_idx, last_best_cost))
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
	if instart >= arr.len() || numiterations < 1 { return Ok(()); }

	// Reset the main cache for the current blocksize.
	state.init_lmc(arr.len() - instart);

	// Greedy run.
	state.greedy(arr, instart, scratch_store, Some(instart))?;

	// Create new stats with the store (updated by the greedy pass).
	let mut current_stats = SymbolStats::new();
	current_stats.load_store(scratch_store);

	// Set up dummy stats we can use to track best and last.
	let mut ran = RanState::new();
	let mut best_stats = SymbolStats::new();

	// We'll also want dummy best and last costs.
	let mut last_cost = NonZeroU32::MIN;
	let mut best_cost = NonZeroU32::MAX;

	// Repeat statistics with the cost model from the previous
	// stat run.
	let mut last_ran = -1;
	for i in 0..numiterations {
		// Optimal run.
		state.optimal_run(arr, instart, &current_stats, scratch_store)?;

		// This is the cost we actually care about.
		let current_cost = calculate_block_size_dynamic(
			scratch_store,
			0..scratch_store.len(),
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



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_fixed_symbols() {
		assert_eq!(
			ArrayLL::<u32>::llcl_symbols(&FIXED_TREE_LL),
			FIXED_SYMBOLS_LL,
		);
		assert_eq!(
			ArrayD::<u32>::llcl_symbols(&FIXED_TREE_D),
			FIXED_SYMBOLS_D,
		);
	}

	#[test]
	fn t_split_idx() {
		// Make sure we have the same number of split indices as we do splits.
		assert_eq!(
			SplitPIdx::all().len(),
			MINIMUM_SPLIT_DISTANCE - 1,
		);

		// Might as well they iterate the same.
		let split1: Vec<usize> = SplitPIdx::all().map(|s| s as usize).collect();
		let split2: Vec<usize> = (0..MINIMUM_SPLIT_DISTANCE - 1).collect();
		assert_eq!(split1, split2);
	}
}
