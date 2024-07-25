/*!
# Flapfli: Blocks.

This module contains the deflate entrypoint and all of the block-related odds
and ends that didn't make it into other modules.
*/

use std::num::{
	NonZeroU32,
	NonZeroUsize,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	DISTANCE_BITS,
	DISTANCE_VALUES,
	DynamicLengths,
	encode_tree,
	FIXED_SYMBOLS_D,
	FIXED_SYMBOLS_LL,
	FIXED_TREE_D,
	FIXED_TREE_LL,
	LENGTH_SYMBOL_BIT_VALUES,
	LENGTH_SYMBOL_BITS,
	LengthLimitedCodeLengths,
	LZ77Store,
	LZ77StoreRange,
	SplitCache,
	SplitLen,
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
/// # Ten is Non-Zero.
const NZ10: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(10) };

#[allow(unsafe_code)]
/// # Eleven is Non-Zero.
const NZ11: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(11) };

/// # Block Split Points.
///
/// This array holds up to fourteen middle points as well as the absolute start
/// and end indices for the chunk/store.
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
	numiterations: NonZeroU32,
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
	let store_len = NonZeroUsize::new(best[best_len as usize + 1]).ok_or(zopfli_error!())?;
	for rng in SplitPointsIter::new(&best, best_len) {
		let rng = rng?;
		let store_rng = store.ranged(rng)?;
		add_lz77_block(
			last_block && rng.end() == store_len.get(),
			store_rng,
			store_len,
			&mut store2,
			state,
			chunk,
			out,
		)?;
	}

	Ok(())
}



#[allow(clippy::cast_precision_loss, clippy::cast_sign_loss)]
#[inline]
/// # Add LZ77 Block (Automatic Type).
///
/// This calculates the expected output sizes for all three block types, then
/// writes the best one to the output file.
fn add_lz77_block(
	last_block: bool,
	store: LZ77StoreRange,
	store_len: NonZeroUsize,
	fixed_store: &mut LZ77Store,
	state: &mut ZopfliState,
	chunk: ZopfliChunk<'_>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	#[inline(never)]
	/// # Add LZ77 Block (Dynamic).
	fn add_dynamic(
		last_block: bool,
		store: LZ77StoreRange,
		out: &mut ZopfliOut,
		extra: u8,
		ll_lengths: &ArrayLL<DeflateSym>,
		d_lengths: &ArrayD<DeflateSym>,
	) -> Result<(), ZopfliError> {
		// Type Bits.
		out.add_header::<BLOCK_TYPE_DYNAMIC>(last_block);

		// Build the lengths first.
		encode_tree(ll_lengths, d_lengths, extra, out)?;

		// Now we need the symbols.
		let ll_symbols = ArrayLL::<u32>::llcl_symbols(ll_lengths);
		let d_symbols = ArrayD::<u32>::llcl_symbols(d_lengths);

		// Write all the data!
		add_lz77_data(store, &ll_symbols, ll_lengths, &d_symbols, d_lengths, out)
	}

	#[inline(never)]
	/// # Add LZ77 Block (Fixed).
	fn add_fixed(
		last_block: bool,
		store: LZ77StoreRange,
		out: &mut ZopfliOut,
	) -> Result<(), ZopfliError> {
		// Type Bits.
		out.add_header::<BLOCK_TYPE_FIXED>(last_block);

		// Write all the data!
		add_lz77_data(
			store,
			&FIXED_SYMBOLS_LL, &FIXED_TREE_LL, &FIXED_SYMBOLS_D, &FIXED_TREE_D,
			out
		)
	}

	#[inline(never)]
	/// # Add Uncompressed.
	///
	/// It is extremely unlikely this will ever be called. Haha.
	fn add_uncompressed(
		last_block: bool,
		store: LZ77StoreRange,
		chunk: ZopfliChunk<'_>,
		out: &mut ZopfliOut,
	) -> Result<(), ZopfliError> {
		let rng = store.byte_range()?;
		let chunk2 = chunk.reslice_rng(rng)?;
		out.add_uncompressed_block(last_block, chunk2);
		Ok(())
	}

	#[inline(never)]
	fn dynamic_details(store: LZ77StoreRange)
	-> Result<DynamicLengths, ZopfliError> { DynamicLengths::new(store) }

	// Calculate the three costs.
	let uncompressed_cost = store.block_size_uncompressed()?;
	let dynamic = dynamic_details(store)?;

	// Most blocks won't benefit from a fixed tree layout, but if we've got a
	// tiny one or the unoptimized-fixed size is within 10% of the dynamic size
	// we should check it out.
	if
		store_len.get() <= LZ77Store::SMALL_STORE ||
		store.block_size_fixed().saturating_mul(NZ10) <= dynamic.cost().saturating_mul(NZ11)
	{
		let rng = store.byte_range()?;
		let fixed_chunk = chunk.reslice_rng(rng)?;
		state.init_lmc(&fixed_chunk);

		// Perform an optimal run.
		state.optimal_run_fixed(fixed_chunk, fixed_store)?;

		// And finally, the cost!
		let fixed_store_rng = fixed_store.ranged_full()?;
		let fixed_cost = fixed_store_rng.block_size_fixed();
		if fixed_cost < dynamic.cost() && fixed_cost <= uncompressed_cost {
			return add_fixed(last_block, fixed_store_rng, out);
		}
	}

	// Dynamic is best!
	if dynamic.cost() <= uncompressed_cost {
		add_dynamic(last_block, store, out, dynamic.extra(), dynamic.ll_lengths(), dynamic.d_lengths())
	}
	// Nothing is everything!
	else {
		add_uncompressed(last_block, store, chunk, out)
	}
}

#[allow(clippy::cast_sign_loss)]
#[inline]
/// # Add LZ77 Data.
///
/// This adds all lit/len/dist codes from the lists as huffman symbols, but not
/// the end code (256).
fn add_lz77_data(
	store: LZ77StoreRange,
	ll_symbols: &ArrayLL<u32>,
	ll_lengths: &ArrayLL<DeflateSym>,
	d_symbols: &ArrayD<u32>,
	d_lengths: &ArrayD<DeflateSym>,
	out: &mut ZopfliOut
) -> Result<(), ZopfliError> {
	for e in store.entries {
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

	// Finish up by writting the end symbol.
	out.add_huffman_bits(ll_symbols[256], ll_lengths[256] as u32);

	Ok(())
}

/// # Calculate Best Block Size (in Bits).
fn calculate_block_size_auto(store: &LZ77Store, rng: ZopfliRange)
-> Result<NonZeroU32, ZopfliError> {
	let small = store.is_small();
	let store = store.ranged(rng)?;
	store.block_size_auto(small)
}

#[inline(never)]
/// # Minimum Split Cost.
///
/// Return the index of the smallest split cost between `start..end`.
fn find_minimum_cost(store: LZ77StoreRange, small: bool)
-> Result<(NonZeroUsize, NonZeroU32), ZopfliError> {
	/// # Split Block Cost.
	///
	/// Sum the left and right halves of the range.
	fn split_cost(a: LZ77StoreRange, b: LZ77StoreRange, small: bool) -> Result<NonZeroU32, ZopfliError> {
		let a = a.block_size_auto(small)?;
		let b = b.block_size_auto(small)?;
		Ok(a.saturating_add(b.get()))
	}

	// Some counters.
	let mut last_best_cost = NonZeroU32::MAX;
	let mut best_split = NonZeroUsize::MAX;

	// Small ranges can just be iterated exhaustively.
	if store.len().get() <= 1024 {
		for (a, b) in store.splits()? {
			let cost = split_cost(a, b, small)?;
			if cost < last_best_cost {
				last_best_cost = cost;
				best_split = a.len(); // The split point.
			}
		}
	}
	// Larger ones require more of a divide-and-conquer approach.
	else {
		let mut splits = store.splits_chunked().ok_or(zopfli_error!())?;
		loop {
			let mut best_cost = NonZeroU32::MAX;
			let mut best_chunk = 0;
			for (i, a, b) in splits.by_ref() {
				let line_cost =
					if best_split == a.len() { last_best_cost }
					else { split_cost(a, b, small)? };

				if i == 0 || line_cost < best_cost {
					best_cost = line_cost;
					best_chunk = i;
				}
			}

			// Stop once we start making things worse.
			if last_best_cost < best_cost { break; }

			// Update the counters.
			best_split = splits.reset(best_chunk);
			last_best_cost = best_cost;
		}
	}

	// If this were going to fail it would have failed a million times over
	// already, but one more check doesn't hurt!
	if best_split < store.len() { Ok((best_split, last_best_cost)) }
	else { Err(zopfli_error!()) }
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
	numiterations: NonZeroU32,
	store: &mut LZ77Store,
	scratch_store: &mut LZ77Store,
	state: &mut ZopfliState,
) -> Result<NonZeroU32, ZopfliError> {
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
	let mut last_cost = NonZeroU32::MAX;
	let mut best_cost = NonZeroU32::MAX;

	// Repeat statistics with the cost model from the previous
	// stat run.
	let mut weighted = false;
	for i in 0..numiterations.get() {
		// Rebuild the symbols.
		current_stats.crunch();

		// Optimal run.
		state.optimal_run(chunk, &current_stats, scratch_store)?;

		// At this point, we only care about the dynamic cost of the chunk.
		let current_cost = scratch_store.ranged_full()
			.and_then(LZ77StoreRange::block_size_dynamic)?;

		// We have a new best!
		if current_cost < best_cost {
			store.replace(scratch_store);
			best_stats = current_stats;
			best_cost = current_cost;
		}

		// Repopulate the counts from the current store, and if the randomness
		// has "warmed up" sufficiently, combine them with half the previous
		// values to create a sort of weighted average.
		current_stats.reload_store(scratch_store, weighted);

		// If nothing changed, replace the current stats with the best stats,
		// reorder the counts, and see what happens.
		if 5 < i && current_cost == last_cost {
			current_stats = best_stats;
			current_stats.randomize(&mut ran);
			weighted = true;
		}
		else { last_cost = current_cost; }
	}

	// Find and return the current (best) cost of the store.
	let store_rng = store.ranged_full()?;
	store_rng.block_size_auto(store_rng.is_small())
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
	numiterations: NonZeroU32,
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
		cost1 += lz77_optimal(
			chunk.reslice(start, end)?,
			numiterations,
			store2,
			&mut store3,
			state,
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
		debug_assert!(split_a[0] == 0); // SplitLen tops out at 14 so we can't actually write to 15 (now 0); it should be the default value, which was zero.
		let mut cost2 = 0;
		for rng in SplitPointsIter::new(&split_a, two_len) {
			cost2 += calculate_block_size_auto(store, rng?)?.get();
		}

		// It's better!
		if cost2 < cost1 { return Ok((split_a, two_len)) }
	}

	split_b.rotate_right(1);
	debug_assert!(split_b[0] == 0); // SplitLen tops out at 14 so we can't actually write to 15 (now 0); it should be the default value, which was zero.
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
				j = j.increment().ok_or(zopfli_error!())?;
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
		lz77size: NonZeroUsize,
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
				else { lz77size.get() - 1 };

			// We found a match!
			if best < end - start && done.is_unset(start) {
				rng.set(start, end)?;
				best = end - start;
			}
		}
		Ok(MINIMUM_SPLIT_DISTANCE <= best)
	}

	// This won't work on tiny files.
	let store = store.ranged_full()?;
	if store.len().get() < MINIMUM_SPLIT_DISTANCE { return Ok(SplitLen::S00); }

	// Get started!
	let mut rng = ZopfliRange::from(store);
	let small = store.is_small(); // Smallness depends on the original store for some reason.
	let done = state.split_cache(rng);
	let mut last = 0;
	let mut len = SplitLen::S00;
	loop {
		let store_rng = store.ranged(rng)?;
		let (llpos, llcost) = find_minimum_cost(store_rng, small)?;

		// Ignore points we've already covered.
		if llpos.get() == 1 || store_rng.block_size_auto(small)? < llcost {
			done.set(rng.start());
		}
		else {
			// The llpos was relative; add it back to start to give it
			// store-wide context.
			let llpos = rng.start() + llpos.get();

			// Mark it as a split point and add it sorted.
			split_b[len as usize] = llpos;
			len = len.increment().ok_or(zopfli_error!())?;

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



/// # Split Range Iterator.
///
/// This iterator converts split points into split ranges, functioning kinda
/// like `slice.windows(2)`, returning between `1..=15` ranges spanning the
/// length of the chunk/store.
struct SplitPointsIter<'a> {
	data: &'a SplitPoints,
	max: SplitLen, // Inclusive length.
	pos: usize,
}

impl<'a> SplitPointsIter<'a> {
	/// # New Instance.
	const fn new(data: &'a SplitPoints, max: SplitLen) -> Self {
		Self { data, max, pos: 0 }
	}
}

impl<'a> Iterator for SplitPointsIter<'a> {
	type Item = Result<ZopfliRange, ZopfliError>;

	fn next(&mut self) -> Option<Self::Item> {
		// The
		if self.pos <= (self.max as usize) {
			let start = self.data[self.pos];
			let end = self.data[self.pos + 1];
			self.pos += 1;
			Some(ZopfliRange::new(start, end))
		}
		else { None }
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for SplitPointsIter<'a> {
	fn len(&self) -> usize {
		(self.max as usize + 1).saturating_sub(self.pos)
	}
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
	fn t_split_points_iter() {
		let data: SplitPoints = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];

		// Try with no mids.
		let mut iter = SplitPointsIter::new(&data, SplitLen::S00);
		assert_eq!(iter.len(), 1);
		let next = iter.next()
			.expect("expected Some(range)")
			.expect("expected Ok(range)");
		assert_eq!(next.rng(), 0..1);
		assert_eq!(iter.len(), 0);
		assert!(iter.next().is_none());

		// Try with two mids.
		iter = SplitPointsIter::new(&data, SplitLen::S02);
		let expected = [0..1_usize, 1..2, 2..3];
		for (i, e) in expected.into_iter().enumerate() {
			assert_eq!(iter.len(), 3 - i);
			let next = iter.next()
				.expect("expected Some(range)")
				.expect("expected Ok(range)");
			assert_eq!(next.rng(), e);
		}
		assert_eq!(iter.len(), 0);
		assert!(iter.next().is_none());
	}
}
