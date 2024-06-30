/*!
# Flapfli: Blocks.

This module contains the deflate entrypoint and all of the block-related odds
and ends that didn't make it into other modules.
*/

use std::num::NonZeroU32;
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
	SplitCache,
	SplitLen,
	SplitPIdx,
	SymbolIteration,
	stats::{
		RanState,
		SymbolStats,
	},
	zopfli_error,
	ZopfliChunk,
	ZopfliError,
	ZopfliOut,
	ZopfliRange,
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

/// # Block Split Points.
///
/// This array holds up to fourteen middle points as well as the absolute start
/// and end indices.
type SplitPoints = [usize; 16];

/// # Zero-Filled Split Points.
const ZEROED_SPLIT_POINTS: SplitPoints = [0; 16];



/// # Deflate a Part.
///
/// Image compression is done in chunks of a million bytes. This does all the
/// work there is to do for one such chunk.
///
/// More specifically, this explores different possible split points for the
/// chunk, then writes the resulting blocks to the output file.
pub(crate) fn deflate_part(
	state: &mut ZopfliState,
	numiterations: i32,
	last_block: bool,
	chunk: ZopfliChunk<'_>,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	let mut store = LZ77Store::new();
	let mut store2 = LZ77Store::new();

	// Find the split points.
	let (best, best_len) = split_points(
		numiterations,
		chunk,
		&mut store,
		&mut store2,
		state,
	)?;

	// Write the data!
	let store_len = best[best_len as usize + 1];
	for pair in best[..best_len as usize + 2].windows(2) {
		let really_last_block = last_block && pair[1] == store_len;

		if let Ok(rng) = ZopfliRange::new(pair[0], pair[1]) {
			add_lz77_block(
				really_last_block,
				&store,
				&mut store2,
				state,
				chunk,
				rng,
				out,
			)?;
		}

		// I don't think empty blocks are possible, but the original zopfli
		// seems to think they imply a fixed-tree layout, so let's run with it!
		else {
			debug_assert_eq!(pair[0], pair[1]);
			out.add_bits(u32::from(really_last_block), 1);
			out.add_bits(1, 2);
			out.add_bits(0, 7);
		}
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
	chunk: ZopfliChunk<'_>,
	rng: ZopfliRange,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	/// # Add LZ77 Block (Dynamic).
	fn add_dynamic(
		last_block: bool,
		store: &LZ77Store,
		rng: ZopfliRange,
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
		rng: ZopfliRange,
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
	fn dynamic_details(store: &LZ77Store, rng: ZopfliRange)
	-> Result<(u8, NonZeroU32, ArrayLL<DeflateSym>, ArrayD<DeflateSym>), ZopfliError> {
		get_dynamic_lengths(store, rng)
	}

	#[inline(never)]
	fn fixed_cost_cold(store: &LZ77Store, rng: ZopfliRange) -> NonZeroU32 {
		calculate_block_size_fixed(store, rng)
	}

	// Calculate the three costs.
	let uncompressed_cost = calculate_block_size_uncompressed(store, rng)?;
	let (dynamic_extra, dynamic_cost, dynamic_ll, dynamic_d) = dynamic_details(store, rng)?;

	// Most blocks won't benefit from a fixed tree layout, but if we've got a
	// tiny one or the unoptimized-fixed size is within 10% of the dynamic size
	// we should check it out.
	if
		store.len() <= 1000 ||
		calculate_block_size_fixed(store, rng).saturating_mul(NZ10) <= dynamic_cost.saturating_mul(NZ11)
	{
		let rng2 = store.byte_range(rng)?;
		let fixed_chunk = chunk.reslice_rng(rng2)?;
		state.init_lmc(&fixed_chunk);

		// Perform an optimal run.
		state.optimal_run_fixed(fixed_chunk, fixed_store)?;

		// And finally, the cost!
		let fixed_rng = ZopfliRange::new(0, fixed_store.len())?;
		let fixed_cost = fixed_cost_cold(fixed_store, fixed_rng);
		if fixed_cost < dynamic_cost && fixed_cost <= uncompressed_cost {
			return add_fixed(last_block, fixed_store, fixed_rng, out);
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
		let chunk2 = chunk.reslice_rng(rng)?;
		out.add_uncompressed_block(last_block, chunk2);
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
	rng: ZopfliRange,
	ll_symbols: &ArrayLL<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_symbols: &ArrayD<u32>,
	d_lengths: &ArrayD<DeflateSym>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	for e in store.entries.get(rng.rng()).ok_or(zopfli_error!())? {
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
fn calculate_block_size_uncompressed(store: &LZ77Store, rng: ZopfliRange)
-> Result<NonZeroU32, ZopfliError> {
	let rng = store.byte_range(rng)?;
	let blocksize = rng.len().get() as u32;

	// Blocks larger than u16::MAX need to be split.
	let blocks = blocksize.div_ceil(65_535);
	NonZeroU32::new(blocks * 40 + blocksize * 8).ok_or(zopfli_error!())
}

/// # Calculate Block Size (Fixed).
fn calculate_block_size_fixed(store: &LZ77Store, rng: ZopfliRange) -> NonZeroU32 {
	// Loop the store if we have data to loop.
	let size = store.entries.get(rng.rng())
		.unwrap_or(&[])
		.iter()
		.map(|e| {
			let mut size = FIXED_TREE_LL[e.ll_symbol as usize] as u32;
			if 0 < e.dist {
				size += u32::from(LENGTH_SYMBOL_BITS[e.litlen as usize]);
				size += u32::from(DISTANCE_BITS[e.d_symbol as usize]);
				size += FIXED_TREE_D[e.d_symbol as usize] as u32;
			}
			size
		})
		.sum::<u32>();

	// This can't really fail, but fixed models are bullshit anyway so we can
	// fall back to an unbeatably large number.
	NonZeroU32::new(size)
		.unwrap_or(NonZeroU32::MAX)
		.saturating_add(FIXED_TREE_LL[256] as u32)
}

/// # Calculate Block Size (Dynamic).
fn calculate_block_size_dynamic(store: &LZ77Store, rng: ZopfliRange)
-> Result<NonZeroU32, ZopfliError> {
	get_dynamic_lengths(store, rng).map(|(_, size, _, _)| size)
}

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto_type(store: &LZ77Store, rng: ZopfliRange)
-> Result<NonZeroU32, ZopfliError> {
	let uncompressed_cost = calculate_block_size_uncompressed(store, rng)?;

	// We can skip the expensive fixed-cost calculations for large blocks since
	// they're unlikely ever to use it.
	let fixed_cost =
		if 1000 < store.len() { uncompressed_cost }
		else { calculate_block_size_fixed(store, rng) };

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
fn find_minimum_cost(store: &LZ77Store, full_rng: ZopfliRange)
-> Result<(usize, NonZeroU32), ZopfliError> {
	#[inline(never)]
	/// # Small Cost.
	///
	/// For small ranges, skip the logic and compare all possible splits. This
	/// will return an error if no splits are possible.
	fn small_cost(store: &LZ77Store, rng: ZopfliRange)
	-> Result<(usize, NonZeroU32), ZopfliError> {
		if 1 < rng.len().get() {
			let mut best_cost = NonZeroU32::MAX;
			let mut best_idx = rng.start() + 1;
			for (a, b) in rng.splits() {
				let cost = split_cost(store, a, b)?;
				if cost < best_cost {
					best_cost = cost;
					best_idx = a.end(); // The split point.
				}
			}
			Ok((best_idx, best_cost))
		}
		// Not big enough to split!
		else { Err(zopfli_error!()) }
	}

	/// # Split Block Cost.
	///
	/// Sum the left and right halves of the range.
	fn split_cost(store: &LZ77Store, a: ZopfliRange, b: ZopfliRange) -> Result<NonZeroU32, ZopfliError> {
		let a = calculate_block_size_auto_type(store, a)?;
		let b = calculate_block_size_auto_type(store, b)?;
		Ok(a.saturating_add(b.get()))
	}

	// Short circuit.
	if full_rng.len().get() <= 1024 { return small_cost(store, full_rng); }

	// Advance the range.
	let mut rng = full_rng.advance()?;

	// Divide and conquer.
	let mut best_cost = NonZeroU32::MAX;
	let mut best_idx = rng.start();
	let mut p = [0_usize; MINIMUM_SPLIT_DISTANCE - 1];
	let mut last_best_cost = NonZeroU32::MAX;
	while MINIMUM_SPLIT_DISTANCE <= rng.len().get() {
		let mut best_p_idx = SplitPIdx::S0;
		for (i, pp) in SplitPIdx::all().zip(p.iter_mut()) {
			*pp = rng.start() + (i as usize + 1) * (rng.len().get().wrapping_div(MINIMUM_SPLIT_DISTANCE));
			let line_cost =
				if best_idx == *pp { last_best_cost }
				else {
					let (a, b) = full_rng.split(*pp)?;
					split_cost(store, a, b)?
				};

			if (i as usize) == 0 || line_cost < best_cost {
				best_cost = line_cost;
				best_p_idx = i;
			}
		}

		// No improvement; we're done.
		if last_best_cost < best_cost { break; }

		// Nudge the boundaries and back again.
		best_idx = p[best_p_idx as usize];
		let new_start =
			if 0 == (best_p_idx as usize) { rng.start() }
			else { p[best_p_idx as usize - 1] };
		let new_end =
			if (best_p_idx as usize) + 1 < p.len() { p[best_p_idx as usize + 1] }
			else { rng.end() };
		rng.set(new_start, new_end)?;

		last_best_cost = best_cost;
	}

	Ok((best_idx, last_best_cost))
}

#[inline]
/// # Optimal LZ77.
///
/// Calculate lit/len and dist pairs for the dataset.
///
/// Note: this incorporates the functionality of `ZopfliLZ77OptimalRun`
/// directly.
fn lz77_optimal(
	chunk: ZopfliChunk<'_>,
	numiterations: i32,
	store: &mut LZ77Store,
	scratch_store: &mut LZ77Store,
	state: &mut ZopfliState,
) -> Result<(), ZopfliError> {
	// Easy abort.
	if numiterations < 1 { return Ok(()); }

	// Reset the main cache for the current blocksize.
	state.init_lmc(&chunk);

	// Greedy run.
	state.greedy(chunk, scratch_store, Some(chunk.pos()))?;

	// Set up the PRNG and two sets of stats, populating one with the greedy-
	// crunched store.
	let mut ran = RanState::new();
	let mut best_stats = SymbolStats::new();
	let mut current_stats = SymbolStats::new();
	current_stats.load_store(scratch_store);

	// We'll also want dummy best and last costs.
	let mut last_cost = NonZeroU32::MIN;
	let mut best_cost = NonZeroU32::MAX;

	// Repeat statistics with the cost model from the previous
	// stat run.
	let mut last_ran = -1;
	for i in 0..numiterations {
		// Rebuild the symbols.
		current_stats.crunch();

		// Optimal run.
		state.optimal_run(chunk, &current_stats, scratch_store)?;

		// This is the cost we actually care about.
		let current_cost = calculate_block_size_dynamic(
			scratch_store,
			ZopfliRange::new(0, scratch_store.len())?,
		)?;

		// We have a new best!
		if current_cost < best_cost {
			store.replace(scratch_store);
			best_stats = current_stats;
			best_cost = current_cost;
		}

		// Repopulate the counts from the current store, and if the randomness
		// has "warmed up" sufficiently, combine them with half the previous
		// values to create a sorted of weighted average.
		current_stats.reload_store(scratch_store, last_ran != -1);

		// If nothing changed, replace the current stats with the best stats,
		// reorder the counts, and see what happens.
		if 5 < i && current_cost == last_cost {
			current_stats = best_stats;
			current_stats.randomize(&mut ran);
			last_ran = i;
		}

		last_cost = current_cost;
	}

	Ok(())
}

#[inline(never)]
/// # Best Split Points.
///
/// Compare the optimal raw and LZ77 split points, returning whichever is
/// predicted to compress better.
///
/// Note the returned length corresponds to the number of points in the middle;
/// it excludes the absolute start and end points.
fn split_points(
	numiterations: i32,
	chunk: ZopfliChunk<'_>,
	store: &mut LZ77Store,
	store2: &mut LZ77Store,
	state: &mut ZopfliState,
) -> Result<(SplitPoints, SplitLen), ZopfliError> {
	// We'll need two sets of split points.
	let mut split_a = ZEROED_SPLIT_POINTS;
	let mut split_b = ZEROED_SPLIT_POINTS;

	// Start by splitting uncompressed.
	let raw_len = split_points_raw(chunk, store2, state, &mut split_a, &mut split_b)?;
	store2.clear();

	// Calculate the costs associated with that split and update the store with
	// the symbol information encountered.
	let mut cost1 = 0;
	let mut store3 = LZ77Store::new();
	for i in 0..=raw_len as usize {
		let start = if i == 0 { chunk.pos() } else { split_a[i - 1] };
		let end = if i < (raw_len as usize) { split_a[i] } else { chunk.total_len().get() };

		// Crunch this chunk into a clean store.
		lz77_optimal(
			chunk.reslice(start, end)?,
			numiterations,
			store2,
			&mut store3,
			state,
		)?;
		cost1 += calculate_block_size_auto_type(
			store2,
			ZopfliRange::new(0, store2.len())?,
		)?.get();

		// Append its data to our main store.
		store.steal_entries(store2);

		// Save the chunk size to our split_b as the defacto best.
		split_b[i] = store.len();
	}

	// If we have at least two split points, do one further LZ77 pass using the
	// updated store details to see if the big picture changes anything.
	if 1 < (raw_len as u8) {
		let two_len = split_points_lz77_cold(state, store, &mut split_a)?;
		split_a[two_len as usize] = store.len();
		split_a.rotate_right(1);
		debug_assert!(split_a[0] == 0); // We don't write to the last byte.
		let mut cost2 = 0;
		for pair in split_a[..two_len as usize + 2].windows(2) {
			cost2 += calculate_block_size_auto_type(
				store,
				ZopfliRange::new(pair[0], pair[1])?,
			)?.get();
		}

		// It's better!
		if cost2 < cost1 { return Ok((split_a, two_len)) }
	}

	split_b.rotate_right(1);
	debug_assert!(split_b[0] == 0); // We don't write to the last byte.
	Ok((split_b, raw_len))
}

#[inline(never)]
/// # Split Points: Uncompressed.
fn split_points_raw(
	chunk: ZopfliChunk<'_>,
	store: &mut LZ77Store,
	state: &mut ZopfliState,
	split_a: &mut SplitPoints,
	split_b: &mut SplitPoints,
) -> Result<SplitLen, ZopfliError> {
	// Populate an LZ77 store from a greedy pass. This results in better
	// block choices than a full optimal pass.
	state.greedy_cold(chunk, store, None)?;

	// Do an LZ77 pass.
	let len = split_points_lz77(state, store, split_b)?;

	// Find the corresponding uncompressed positions.
	if len.is_zero() { Ok(len) }
	else {
		let mut pos = chunk.pos();
		let mut j = SplitLen::S00;
		for (i, e) in store.entries.iter().enumerate().take(split_b[len as usize - 1] + 1) {
			if i == split_b[j as usize] {
				split_a[j as usize] = pos;
				j = j.increment();
				if (j as u8) == (len as u8) { return Ok(len); }
			}
			pos += e.length() as usize;
		}

		Err(zopfli_error!())
	}
}

#[inline(never)]
fn split_points_lz77_cold(
	state: &mut ZopfliState,
	store: &LZ77Store,
	split_b: &mut SplitPoints,
) -> Result<SplitLen, ZopfliError> { split_points_lz77(state, store, split_b) }

#[inline]
/// # LZ77 Split Pass.
///
/// This sets the LZ77 split points according to convoluted cost
/// evaluations.
fn split_points_lz77(
	state: &mut ZopfliState,
	store: &LZ77Store,
	split_b: &mut SplitPoints,
) -> Result<SplitLen, ZopfliError> {
	/// # Find Largest Splittable Block.
	///
	/// This finds the largest available block for splitting, evenly spreading the
	/// load if a limited number of blocks are requested.
	///
	/// Returns `false` if no blocks are found.
	fn find_largest(
		lz77size: usize,
		done: &SplitCache,
		splitpoints: &[usize],
		rng: &mut ZopfliRange,
	) -> Result<bool, ZopfliError> {
		let mut best = 0;
		for i in 0..=splitpoints.len() {
			let start =
				if i == 0 { 0 }
				else { splitpoints[i - 1] };
			let end =
				if i < splitpoints.len() { splitpoints[i] }
				else { lz77size - 1 };

			// We found a match!
			if best < end - start && done.is_unset(start) {
				rng.set(start, end)?;
				best = end - start;
			}
		}
		Ok(MINIMUM_SPLIT_DISTANCE <= best)
	}

	// This won't work on tiny files.
	if store.len() < MINIMUM_SPLIT_DISTANCE { return Ok(SplitLen::S00); }

	// Get started!
	let mut rng = ZopfliRange::new(0, store.len())?;
	let done = state.split_cache(rng);
	let mut last = 0;
	let mut len = SplitLen::S00;
	loop {
		// Safety: find_minimum_cost will return an error if the block doesn't
		// have a midpoint.
		let (llpos, llcost) = find_minimum_cost(store, rng)?;
		if rng.start() >= llpos || rng.end() <= llpos { crate::unreachable(); }

		// Ignore points we've already covered.
		if llpos == rng.start() + 1 || calculate_block_size_auto_type(store, rng)? < llcost {
			done.set(rng.start());
		}
		else {
			// Mark it as a split point and add it sorted.
			split_b[len as usize] = llpos;
			len = len.increment();

			// Keep the list sorted.
			if last > llpos { split_b[..len as usize].sort_unstable(); }
			else { last = llpos; }

			// Stop if we've split the maximum number of times.
			if len.is_max() { break; }
		}

		// Look for a split and adjust the start/end accordingly. If we don't
		// find one or the remaining distance is too small to continue, we're
		// done!
		if ! find_largest(
			store.len(),
			done,
			&split_b[..len as usize],
			&mut rng,
		)? { break; }
	}

	Ok(len)
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
