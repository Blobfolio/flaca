/*!
# Flapfli: Matches and Hashes.

This module contains the zopfli match-hashing functionality, along with a
collective structure bundling all the hash/cache shit together.
*/

use std::{
	alloc::{
		alloc_zeroed,
		handle_alloc_error,
		Layout,
	},
	cell::Cell,
	ptr::{
		addr_of_mut,
		NonNull,
	},
};
use super::{
	DISTANCE_BITS_F,
	DISTANCE_SYMBOLS,
	LENGTH_SYMBOL_BITS_F,
	LENGTH_SYMBOLS,
	LitLen,
	LZ77Store,
	MatchCache,
	ReducingSlices,
	SplitCache,
	SqueezeCache,
	stats::SymbolStats,
	SUBLEN_LEN,
	zopfli_error,
	ZOPFLI_MAX_MATCH,
	ZOPFLI_MIN_MATCH,
	ZOPFLI_WINDOW_SIZE,
	ZopfliChunk,
	ZopfliError,
	ZopfliRange,
};

const ZOPFLI_WINDOW_MASK: usize = ZOPFLI_WINDOW_SIZE - 1;
const HASH_SHIFT: i32 = 5;
const HASH_MASK: i16 = 32_767;
const ZOPFLI_MAX_CHAIN_HITS: usize = 8192;

/// # Distance Bits (for minimum cost).
const MIN_COST_DISTANCES: [u8; 30] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3,
	4, 4, 5, 5, 6, 6, 7, 7, 8, 8,
	9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];

/// # Zero-Filled Sublength Cache.
const ZEROED_SUBLEN: [u16; SUBLEN_LEN] = [0; SUBLEN_LEN];



/// # Zopfli State.
///
/// This consolidates the Longest Match, Squeeze, Split, and Hash caches into a
/// single gratuitous structure, cutting down on the number of references we
/// need to bounce from method to method.
///
/// Each member is big and terrible in its own right, but on the bright side we
/// only need a single instance per thread for the duration of the program run,
/// so the allocations are a one-and-done affair.
///
/// (That local lives in `deflate.rs`.)
pub(crate) struct ZopfliState {
	lmc: MatchCache,
	hash: ZopfliHash,
	split: SplitCache,
	squeeze: SqueezeCache,
}

impl ZopfliState {
	#[allow(unsafe_code)]
	#[inline(never)]
	/// # New.
	///
	/// This struct's members are mostly large and terrible arrays. To keep
	/// them off the stack, it is necessary to initialize everything from raw
	/// pointers and box them up.
	///
	/// This unfortunately requires a some upfront unsafe code during
	/// construction, but everything can be accessed normally thereafter.
	///
	/// To cut down on some of the complexity, the manual layout allocation and
	/// boxing is done once, here, instead of separately for each individual
	/// member.
	pub(crate) fn new() -> Box<Self> {
		// Reserve the space.
		const LAYOUT: Layout = Layout::new::<ZopfliState>();
		let out: NonNull<Self> = NonNull::new(unsafe { alloc_zeroed(LAYOUT).cast() })
			.unwrap_or_else(|| handle_alloc_error(LAYOUT));
		let ptr = out.as_ptr();

		unsafe {
			// Safety: zeroes are "valid" for all of the primitives — including
			// LitLen, which is sized/aligned to u16 —  so alloc_zeroed has
			// taken care of everything but the Cell in SqueezeCache, which we
			// can sort out thusly:
			addr_of_mut!((*ptr).squeeze.costs_len).write(Cell::new(0));

			// Note: zero is not the appropriate _logical_ default in most
			// cases, but since this struct is designed for reuse, it manually
			// resets everything when starting a new cycle anyway. At this
			// stage, validity is sufficient.

			// Done!
			Box::from_raw(ptr)
		}
	}

	/// # Initialize LMC/Squeeze Caches.
	///
	/// This prepares the Longest Match Cache and Squeeze caches for subsequent
	/// work on `chunk`, if any.
	pub(crate) fn init_lmc(&mut self, chunk: &ZopfliChunk<'_>) {
		self.lmc.init(chunk);
		self.squeeze.resize_costs(chunk);
	}

	/// # Split Cache.
	///
	/// Clear the split cache and return a mutable reference to it so the
	/// split points within `rng` can be tracked.
	pub(crate) fn split_cache(&mut self, rng: ZopfliRange) -> &mut SplitCache {
		self.split.init(rng);
		&mut self.split
	}
}

impl ZopfliState {
	#[inline(never)]
	/// # Greedy LZ77 Run (No Inlining).
	///
	/// Same as `greedy`, but the compiler is given an `inline(never)` hint to
	/// (hopefully) keep all this code from affecting its inlining decisions
	/// about the caller.
	pub(crate) fn greedy_cold(
		&mut self,
		chunk: ZopfliChunk<'_>,
		store: &mut LZ77Store,
		cache: Option<usize>,
	) -> Result<(), ZopfliError> {
		self.greedy(chunk, store, cache)
	}

	#[allow(clippy::cast_possible_truncation)]
	#[inline]
	/// # Greedy LZ77 Run.
	///
	/// This method looks for best-length matches in the data (and/or cache),
	/// updating the store with the results.
	///
	/// This is very similar to `ZopfliState::optimal_run`, but better suited
	/// for general-purpose store population.
	pub(crate) fn greedy(
		&mut self,
		chunk: ZopfliChunk<'_>,
		store: &mut LZ77Store,
		cache: Option<usize>,
	) -> Result<(), ZopfliError> {
		/// # Distance-Based Length Score.
		const fn get_length_score(length: LitLen, distance: u16) -> u16 {
			if 1024 < distance { (length as u16).saturating_sub(1) }
			else { length as u16 }
		}

		// Reset the store and hash.
		store.clear();
		self.hash.reset(chunk);

		// Short circuit.
		let mut iter = chunk.reducing_block_iter();

		// We'll need a few more variables…
		let mut sublen = ZEROED_SUBLEN;
		let mut length = LitLen::L000;
		let mut distance: u16 = 0;
		let mut prev_length = LitLen::L000;
		let mut prev_distance: u16 = 0;
		let mut match_available = false;
		let mut prev_value = 0_u8;

		// Loop the data!
		while let Some(chunk2) = iter.next() {
			self.hash.update_hash(chunk2);
			let prev_prev_value = std::mem::replace(&mut prev_value, chunk2.first());

			// Run the finder.
			self.hash.find(
				chunk2,
				LitLen::MAX_MATCH,
				&mut Some(&mut sublen),
				&mut distance,
				&mut length,
				&mut self.lmc,
				cache,
			)?;

			// Lazy matching.
			let length_score = get_length_score(length, distance);
			let prev_length_score = get_length_score(prev_length, prev_distance);
			if match_available {
				match_available = false;

				if length_score > prev_length_score + 1 {
					store.push(
						LitLen::from_u8(prev_prev_value),
						0,
						chunk2.pos() - 1,
					);
					if length_score >= ZOPFLI_MIN_MATCH as u16 && ! length.is_max() {
						match_available = true;
						prev_length = length;
						prev_distance = distance;
						continue;
					}
				}
				else {
					// Old is new.
					length = prev_length;
					distance = prev_distance;

					// Write the values!
					store.push(length, distance, chunk2.pos() - 1);

					// Update the hash up through length and increment the loop
					// position accordingly.
					for chunk2 in iter.by_ref().take(length as usize - 2) {
						self.hash.update_hash(chunk2);
					}
					continue;
				}
			}
			// No previous match, but maybe we can set it for the next
			// iteration?
			else if length_score >= ZOPFLI_MIN_MATCH as u16 && ! length.is_max() {
				match_available = true;
				prev_length = length;
				prev_distance = distance;
				continue;
			}

			// Write the current length/distance.
			if length_score >= ZOPFLI_MIN_MATCH as u16 {
				store.push(length, distance, chunk2.pos());
			}
			// Write from the source with no distance and reset the length to
			// one.
			else {
				length = LitLen::L001;
				store.push(LitLen::from_u8(chunk2.first()), 0, chunk2.pos());
			}

			// Update the hash up through length and increment the loop
			// position accordingly.
			for chunk2 in iter.by_ref().take(length as usize - 1) {
				self.hash.update_hash(chunk2);
			}
		}

		Ok(())
	}

	#[inline(never)]
	/// # Optimal Run (Fixed).
	///
	/// Same as `ZopfliHash::optimal_run`, but fixed tree counts and symbols
	/// are used instead of the store's actual histogram.
	pub(crate) fn optimal_run_fixed(
		&mut self,
		chunk: ZopfliChunk<'_>,
		store: &mut LZ77Store,
	) -> Result<(), ZopfliError> {
		// Reset the store and costs.
		store.clear();
		let costs = self.squeeze.reset_costs();
		if ! costs.is_empty() {
			// Reset and warm the hash.
			self.hash.reset(chunk);

			// Forward and backward squeeze passes.
			self.hash.get_best_lengths_fixed(chunk, costs, &mut self.lmc)?;
			let paths = self.squeeze.trace_paths()?;
			if ! paths.is_empty() {
				self.hash.follow_paths(
					chunk,
					paths,
					store,
					&mut self.lmc,
				)?;
			}
		}

		Ok(())
	}

	#[inline(never)]
	/// # Optimal Run.
	///
	/// This performs backward/forward squeeze passes on the data with
	/// existing histogram data. The `store` is updated with the best-length
	/// match data.
	pub(crate) fn optimal_run(
		&mut self,
		chunk: ZopfliChunk<'_>,
		stats: &SymbolStats,
		store: &mut LZ77Store,
	) -> Result<(), ZopfliError> {
		// Reset the store and costs.
		store.clear();
		let costs = self.squeeze.reset_costs();
		if ! costs.is_empty() {
			// Reset and warm the hash.
			self.hash.reset(chunk);

			// Forward and backward squeeze passes.
			self.hash.get_best_lengths(chunk, stats, costs, &mut self.lmc)?;
			let paths = self.squeeze.trace_paths()?;
			if ! paths.is_empty() {
				self.hash.follow_paths(
					chunk,
					paths,
					store,
					&mut self.lmc,
				)?;
			}
		}

		Ok(())
	}
}



#[derive(Clone, Copy)]
/// # Zopfli Hash.
///
/// This structure tracks byte values and hashes by position, facilitating
/// match-finding (length and distance) at various offsets.
struct ZopfliHash {
	chain1: ZopfliHashChain,
	chain2: ZopfliHashChain,

	/// # Repetitions of the same byte after this.
	same: [u16; ZOPFLI_WINDOW_SIZE],
}

impl ZopfliHash {
	/// # Reset/Warm Up.
	///
	/// This sets all values to their defaults, then cycles the first chain's
	/// hash value once or twice, then hashes the bits between the start of the
	/// window and the start of the slice we're actually interested in, if any.
	fn reset(&mut self, chunk: ZopfliChunk<'_>) {
		// Reset the data.
		self.chain1.reset();
		self.chain2.reset();
		self.same.fill(0);

		// Cycle the hash once or twice.
		let (a, b) = chunk.warmup_values();
		self.update_hash_value(a);
		if let Some(b) = b { self.update_hash_value(b); }

		// Process the values between windowstart and instart, if any.
		if let Some(iter) = chunk.reducing_prelude_iter() {
			for chunk2 in iter { self.update_hash(chunk2); }
		}
	}

	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::similar_names,
	)]
	/// # Update Hash.
	///
	/// This updates the hash tables using the chunk's block data.
	fn update_hash(&mut self, chunk: ZopfliChunk<'_>) {
		let pos = chunk.pos();
		let hpos = pos & ZOPFLI_WINDOW_MASK;

		// Cycle the first hash.
		let arr = chunk.block();
		self.update_hash_value(arr.get(ZOPFLI_MIN_MATCH - 1).map_or(0, |v| *v));
		self.chain1.update_hash(pos);

		// Count up the repetitions (and update sameness).
		let current = chunk.first();
		let mut amount = self.same[pos.wrapping_sub(1) & ZOPFLI_WINDOW_MASK]
			.saturating_sub(1);
		while
			amount < u16::MAX &&
			usize::from(amount) + 1 < arr.len() &&
			current == arr[usize::from(amount) + 1]
		{
			amount += 1;
		}
		self.same[hpos] = amount;

		// Cycle the second hash.
		self.chain2.val = ((amount.wrapping_sub(ZOPFLI_MIN_MATCH as u16) & 255) as i16) ^ self.chain1.val;
		self.chain2.update_hash(pos);
	}

	/// # Update Hash Value.
	///
	/// This updates the rotating (chain1) hash value.
	///
	/// Note: the value will always fit within the equivalent of `u15`.
	fn update_hash_value(&mut self, c: u8) {
		self.chain1.val = ((self.chain1.val << HASH_SHIFT) ^ i16::from(c)) & HASH_MASK;
	}
}

impl ZopfliHash {
	#[allow(clippy::cast_possible_truncation)]
	#[inline(never)]
	/// # Get Best Lengths.
	///
	/// This method performs the forward pass for "squeeze", calculating the
	/// optimal length to reach every byte from a previous byte. The resulting
	/// cost is returned.
	///
	/// Note: the costs really do need to be calculated in 64 bits, truncated
	/// to 32 bits for storage, then widened back to 64 bits for comparison.
	/// Zopfli is evil!
	fn get_best_lengths(
		&mut self,
		chunk: ZopfliChunk<'_>,
		stats: &SymbolStats,
		costs: &mut [(f32, LitLen)],
		lmc: &mut MatchCache,
	) -> Result<(), ZopfliError> {
		/// # Minimum Cost Model (Non-Zero Distances).
		fn minimum_cost(stats: &SymbolStats) -> f64 {
			// Find the minimum length cost.
			let mut length_cost = f64::INFINITY;
			for (lsym, lbits) in LENGTH_SYMBOLS.iter().copied().zip(LENGTH_SYMBOL_BITS_F.into_iter()).skip(3) {
				let cost = lbits + stats.ll_symbols[lsym as usize];
				if cost < length_cost { length_cost = cost; }
			}

			// Now find the minimum distance cost.
			let mut dist_cost = f64::INFINITY;
			for (bits, v) in MIN_COST_DISTANCES.iter().copied().zip(stats.d_symbols) {
				let cost = f64::from(bits) + v;
				if cost < dist_cost { dist_cost = cost; }
			}

			// Add them together and we have our minimum.
			length_cost + dist_cost
		}

		/// # Adjusted Cost.
		fn stat_cost(dist: u16, k: LitLen, stats: &SymbolStats) -> f64 {
			if dist == 0 { stats.ll_symbols[k as usize] }
			else {
				let dsym = DISTANCE_SYMBOLS[(dist & 32_767) as usize];
				DISTANCE_BITS_F[dsym as usize] +
				stats.d_symbols[dsym as usize] +
				stats.ll_symbols[LENGTH_SYMBOLS[k as usize] as usize] +
				LENGTH_SYMBOL_BITS_F[k as usize]
			}
		}

		// The costs are sized according to the (relevant) array slice; they
		// should always be exactly one larger.
		if costs.len() != chunk.block_size().get() + 1 {
			return Err(zopfli_error!());
		}

		// Iterators will help us avoid a bunch of unsafe.
		let instart = chunk.pos();
		let mut iter = chunk.reducing_block_iter().zip(
			ReducingSlices::new(Cell::from_mut(costs).as_slice_of_cells())
		);

		let mut length = LitLen::L000;
		let mut distance = 0_u16;
		let mut sublen = ZEROED_SUBLEN;

		// Find the minimum and symbol costs, which we'll need to reference
		// repeatedly in the loop.
		let min_cost = minimum_cost(stats);
		let symbol_cost = stats.ll_symbols[285] + stats.d_symbols[0];

		while let Some((mut chunk2, mut cost2)) = iter.next() {
			debug_assert_eq!(chunk2.block_size().get() + 1, cost2.len());

			// Hash the remainder.
			self.update_hash(chunk2);

			let pos = chunk2.pos();
			if
				// We have more than ZOPFLI_MAX_MATCH entries behind us, and twice
				// twice as many ahead of us.
				pos > instart + ZOPFLI_MAX_MATCH + 1 &&
				chunk2.block_size().get() > ZOPFLI_MAX_MATCH * 2 + 1 &&
				// The current and max-match-ago positions have long repetitions.
				self.same[pos & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16 * 2 &&
				self.same[(pos - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16
			{
				// Fast forward!
				let before = pos;
				for (chunk3, cost3) in iter.by_ref().take(ZOPFLI_MAX_MATCH) {
					// Safety: arr2.len() has at least ZOPFLI_MAX_MATCH*2+1
					// remaining entries; cost2.len() will be at least one
					// more than that.
					if cost2.len() <= ZOPFLI_MAX_MATCH { crate::unreachable(); }
					cost2[ZOPFLI_MAX_MATCH].set((
						(f64::from(cost2[0].get().0) + symbol_cost) as f32,
						LitLen::MAX_MATCH,
					));
					cost2 = cost3; // The costs are rotated _after_ updating…

					chunk2 = chunk3;   // …but the array is rotated beforehand.
					self.update_hash(chunk2);
				}

				debug_assert_eq!(chunk2.pos() - before, ZOPFLI_MAX_MATCH);
				debug_assert_eq!(chunk2.block_size().get() + 1, cost2.len());
			}

			// Find the longest remaining match.
			self.find(
				chunk2,
				LitLen::MAX_MATCH,
				&mut Some(&mut sublen),
				&mut distance,
				&mut length,
				lmc,
				Some(instart),
			)?;

			// Safety: the MAX loop (if it ran at all) only advanced the
			// slices ZOPFLI_MAX_MATCH; we have more work to do!
			if cost2.len() < 2 { crate::unreachable(); }

			// Update it if lower.
			let cost_j = f64::from(cost2[0].get().0);
			let new_cost = stats.ll_symbols[usize::from(chunk2.first())] + cost_j;
			if new_cost < f64::from(cost2[1].get().0) {
				cost2[1].set((new_cost as f32, LitLen::L001));
			}

			// If a long match was found, peek forward to recalculate those
			// costs, at least the ones who could benefit from the expense of
			// all that effort.
			let limit = length.min_usize(cost2.len() - 1);
			if limit.is_matchable() {
				let min_cost_add = min_cost + cost_j;

				// Safety: limit is capped to cost2.len() - 1.
				if cost2.len() <= (limit as usize) { crate::unreachable(); }

				for ((dist, c), k) in sublen[ZOPFLI_MIN_MATCH..=limit as usize].iter()
					.copied()
					.zip(&cost2[ZOPFLI_MIN_MATCH..=limit as usize])
					.zip(LitLen::matchable_iter())
				{
					let current_cost = f64::from(c.get().0);
					if min_cost_add < current_cost {
						// Update it if lower.
						let new_cost = cost_j + stat_cost(dist, k, stats);
						if new_cost < current_cost { c.set((new_cost as f32, k)); }
					}
				}
			}
		}

		// All costs should have been updated…
		debug_assert!(costs.iter().all(|(cost, _)| (0.0..1E30).contains(cost)));
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	#[inline(never)]
	/// # Get Best Lengths (Fixed).
	///
	/// Same as `ZopfliHash::get_best_lengths`, but simpler fixed-tree lengths
	/// and symbols are used instead of variable store-specific data.
	fn get_best_lengths_fixed(
		&mut self,
		chunk: ZopfliChunk<'_>,
		costs: &mut [(f32, LitLen)],
		lmc: &mut MatchCache,
	) -> Result<(), ZopfliError> {
		/// # Adjusted Cost.
		///
		/// These are really tiny so we might as well use single-byte math.
		const fn fixed_cost(dist: u16, k: LitLen) -> u8 {
			use super::{DISTANCE_BITS, LENGTH_SYMBOL_BITS};

			if dist == 0 { 8 + (143 < (k as u16)) as u8 }
			else {
				let dsym = DISTANCE_SYMBOLS[(dist & 32_767) as usize];
				DISTANCE_BITS[dsym as usize] +
				LENGTH_SYMBOL_BITS[k as usize] +
				(114 < (k as u16)) as u8 +
				12
			}
		}

		// The costs are sized according to the (relevant) array slice; they
		// should always be exactly one larger.
		if costs.len() != chunk.block_size().get() + 1 {
			return Err(zopfli_error!());
		}

		// Iterators will help us avoid a bunch of unsafe.
		let instart = chunk.pos();
		let mut iter = chunk.reducing_block_iter().zip(
			ReducingSlices::new(Cell::from_mut(costs).as_slice_of_cells())
		);

		let mut length = LitLen::L000;
		let mut distance = 0_u16;
		let mut sublen = ZEROED_SUBLEN;

		while let Some((mut chunk2, mut cost2)) = iter.next() {
			debug_assert_eq!(chunk2.block_size().get() + 1, cost2.len());

			// Hash the remainder.
			self.update_hash(chunk2);

			let pos = chunk2.pos();
			if
				// We have more than ZOPFLI_MAX_MATCH entries behind us, and twice
				// twice as many ahead of us.
				pos > instart + ZOPFLI_MAX_MATCH + 1 &&
				chunk2.block_size().get() > ZOPFLI_MAX_MATCH * 2 + 1 &&
				// The current and max-match-ago positions have long repetitions.
				self.same[pos & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16 * 2 &&
				self.same[(pos - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16
			{
				// Fast forward!
				let before = pos;
				for (chunk3, cost3) in iter.by_ref().take(ZOPFLI_MAX_MATCH) {
					// Safety: arr2.len() has at least ZOPFLI_MAX_MATCH*2+1
					// remaining entries; cost2.len() will be at least one
					// more than that.
					if cost2.len() <= ZOPFLI_MAX_MATCH { crate::unreachable(); }
					cost2[ZOPFLI_MAX_MATCH].set((
						(f64::from(cost2[0].get().0) + 13.0) as f32,
						LitLen::MAX_MATCH,
					));
					cost2 = cost3; // The costs are rotated _after_ updating…

					chunk2 = chunk3;   // …but the array is rotated beforehand.
					self.update_hash(chunk2);
				}

				debug_assert_eq!(chunk2.pos() - before, ZOPFLI_MAX_MATCH);
				debug_assert_eq!(chunk2.block_size().get() + 1, cost2.len());
			}

			// Find the longest remaining match.
			self.find(
				chunk2,
				LitLen::MAX_MATCH,
				&mut Some(&mut sublen),
				&mut distance,
				&mut length,
				lmc,
				Some(instart),
			)?;

			// Safety: the MAX loop (if it ran at all) only advanced the
			// slices ZOPFLI_MAX_MATCH; we have more work to do!
			if cost2.len() < 2 { crate::unreachable(); }

			// Update it if lower.
			let cost_j = f64::from(cost2[0].get().0);
			let new_cost = if chunk2.first() <= 143 { 8.0 } else { 9.0 } + cost_j;
			if new_cost < f64::from(cost2[1].get().0) {
				cost2[1].set((new_cost as f32, LitLen::L001));
			}

			// If a long match was found, peek forward to recalculate those
			// costs, at least the ones who could benefit from the expense of
			// all that effort.
			let limit = length.min_usize(cost2.len() - 1);
			if limit.is_matchable() {
				let min_cost_add = 8.0 + cost_j;

				// Safety: limit is capped to cost2.len() - 1.
				if cost2.len() <= (limit as usize) { crate::unreachable(); }

				for ((dist, c), k) in sublen[ZOPFLI_MIN_MATCH..=limit as usize].iter()
					.copied()
					.zip(&cost2[ZOPFLI_MIN_MATCH..=limit as usize])
					.zip(LitLen::matchable_iter())
				{
					let current_cost = f64::from(c.get().0);
					if min_cost_add < current_cost {
						// Update it if lower.
						let new_cost = cost_j + f64::from(fixed_cost(dist, k));
						if new_cost < current_cost { c.set((new_cost as f32, k)); }
					}
				}
			}
		}

		// All costs should have been updated…
		debug_assert!(costs.iter().all(|(cost, _)| (0.0..1E30).contains(cost)));
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Follow Paths.
	///
	/// This method repopulates the hash tables by following the provided
	/// squeeze-based path lengths. The store is updated with the results.
	fn follow_paths(
		&mut self,
		chunk: ZopfliChunk<'_>,
		paths: &[LitLen],
		store: &mut LZ77Store,
		lmc: &mut MatchCache,
	) -> Result<(), ZopfliError> {
		// Reset the hash.
		self.reset(chunk);

		// Hash the path symbols.
		let instart = chunk.pos();
		let mut len_iter = paths.iter().copied();
		let mut arr_iter = chunk.reducing_block_iter();
		while let Some((length, chunk2)) = len_iter.next().zip(arr_iter.next()) {
			// Hash it.
			self.update_hash(chunk2);

			// Follow the matches!
			if length.is_matchable() {
				// Get the distance by recalculating the longest match, and
				// make sure the length matches afterwards (as that's easy to
				// screw up!).
				let mut test_length = LitLen::L000;
				let mut dist = 0;
				self.find(
					chunk2,
					length,
					&mut None,
					&mut dist,
					&mut test_length,
					lmc,
					Some(instart),
				)?;

				// Make sure we were able to find what we were expecting.
				if test_length != length && test_length.is_matchable() {
					return Err(zopfli_error!());
				}

				// Add it to the store.
				store.push(length, dist, chunk2.pos());

				// Hash the rest of the match.
				for chunk2 in arr_iter.by_ref().take(length as usize - 1) {
					self.update_hash(chunk2);
				}
			}
			// It isn't matchable; add it directly to the store.
			else {
				store.push(LitLen::from_u8(chunk2.first()), 0, chunk2.pos());
			}
		}

		Ok(())
	}
}

impl ZopfliHash {
	#[allow(clippy::too_many_arguments)]
	/// # Find Longest Match.
	///
	/// This finds the longest match in the chunk (and/or the cache), setting
	/// the provided `sublen`/`distance`/`length` values accordingly.
	///
	/// Lengths will never exceed `limit` nor `ZOPFLI_MAX_MATCH`, but they
	/// might be _less_ than `ZOPFLI_MIN_MATCH`, especially as we near the end
	/// of the block slice.
	fn find(
		&self,
		chunk: ZopfliChunk<'_>,
		mut limit: LitLen,
		sublen: &mut Option<&mut [u16; SUBLEN_LEN]>,
		distance: &mut u16,
		length: &mut LitLen,
		lmc: &mut MatchCache,
		cache: Option<usize>,
	) -> Result<(), ZopfliError> {
		// Check the longest match cache first!
		let pos = chunk.pos();
		if let Some(blockstart) = cache {
			if lmc.find(
				pos - blockstart,
				&mut limit,
				sublen,
				distance,
				length,
			)? {
				if (*length as usize) <= chunk.block_size().get() { return Ok(()); }
				return Err(zopfli_error!());
			}
		}

		// We'll need at least ZOPFLI_MIN_MATCH bytes for a search; if we don't
		// have it, zero everything out and call it a day.
		if ZOPFLI_MIN_MATCH > chunk.block_size().get() {
			*length = LitLen::L000;
			*distance = 0;
			return Ok(());
		}

		// Cap the limit to fit if needed. Note that limit will always be at
		// least one even if capped since pos < size.
		limit = limit.min_usize(chunk.block_size().get());

		// Calculate the best distance and length.
		let (bestdist, bestlength) = self.find_loop(chunk, limit, sublen);

		// Cache the results for next time, maybe.
		if limit.is_max() {
			if let Some(blockstart) = cache {
				if let Some(s) = sublen {
					lmc.set_sublen(pos - blockstart, s, bestdist, bestlength)?;
				}
			}
		}

		// Update the values.
		*distance = bestdist;
		*length = bestlength;
		if (*length as usize) <= chunk.block_size().get() { Ok(()) }
		else { Err(zopfli_error!()) }
	}

	#[allow(
		unsafe_code,
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
		clippy::similar_names,
	)]
	/// # Find Longest Match Loop.
	///
	/// This method is a (nasty-looking) workhorse for the above
	/// `ZopfliHash::find` method. It finds and returns the matching distance
	/// and length, or `(0, 1)` if none.
	fn find_loop(
		&self,
		chunk: ZopfliChunk<'_>,
		limit: LitLen,
		sublen: &mut Option<&mut [u16; SUBLEN_LEN]>,
	) -> (u16, LitLen) {
		/// # Distance Given Positions.
		const fn ppp_distance(p: usize, pp: usize) -> usize {
			if p < pp { pp - p }
			else { ZOPFLI_WINDOW_SIZE + pp - p }
		}

		// Prepopulate some slices to work with directly later on.
		let arr = chunk.arr();
		let right = chunk.block();

		let pos = chunk.pos();
		let hpos = pos & ZOPFLI_WINDOW_MASK;

		// The default distance and length. We'll be wanting 16-bit values for
		// both eventually, but they're used in a lot of indexing so usize is
		// more ergonomical for now.
		let mut bestdist: usize = 0;
		let mut bestlength = LitLen::L001;

		// We'll start by looking at the first hash chain, but may switch
		// midway through if the second chain is better.
		let mut switched = false;
		let mut chain = &self.chain1;

		debug_assert_eq!(chain.hash_idx[chain.val as usize], hpos as i16);

		// Keep track of the current and previous matches, if any.
		let mut pp = hpos;
		let mut p =
			if chain.idx_prev[hpos] < 0 { hpos }
			else { chain.idx_prev[hpos] as usize };

		// Even though the ultimate distance will be u16, this variable needs
		// to be at least 32-bit to keep the math from overflowing.
		let mut dist = ppp_distance(p, pp);
		let mut hits = 0;
		let same0 = self.same[hpos];
		let same1 = limit.min_u16(same0);
		while p < ZOPFLI_WINDOW_SIZE && dist < ZOPFLI_WINDOW_SIZE && hits < ZOPFLI_MAX_CHAIN_HITS {
			// These are simple sanity assertions; the values are only ever
			// altered via ZopfliHashChain::update_hash so there isn't much
			// room for mistake.
			debug_assert!(p as i16 == chain.idx_prev[pp] || p == pp);
			debug_assert_eq!(chain.idx_hash[p], chain.val);

			// If we have distance, we can check the length of the match. The
			// logic here is extremely convoluted, but essentially we'll
			// always wind up with a value between 1..=ZOPFLI_MAX_MATCH. More
			// specifically:
			// * bestlength starts at 1 so nothing happens if we find 0.
			// * (pos + limit <= arr.len())
			// * (limit <= ZOPFLI_MAX_MATCH)
			// * (same <= limit), so…
			// * (pos + same <= arr.len()) too
			if 0 != dist && dist <= pos {
				// Safety: we (safely) sliced right to arr[pos..] earlier and
				// verified it was non-empty, but the compiler will have
				// forgotten that by now.
				let left = unsafe { arr.get_unchecked(pos - dist..pos - dist + right.len()) };
				if right.is_empty() || left.len() != right.len() { crate::unreachable(); }

				// Check to see if we can do better than we've already done.
				if (bestlength as usize) >= right.len() || right[bestlength as usize] == left[bestlength as usize] {
					// Check the match cache to see if we can start later.
					let mut currentlength =
						if 2 < same0 && right[0] == left[0] {
							same1.min_u16(self.same[(pos - dist) & ZOPFLI_WINDOW_MASK])
						}
						else { LitLen::L000 };

					// Bump the length for each matching left/right pair, up to
					// the limit.
					for next in LitLen::next_iter(currentlength).take((limit as usize) - (currentlength as usize)) {
						if
							(currentlength as usize) < right.len() &&
							left[currentlength as usize] == right[currentlength as usize]
						{
							currentlength = next;
						}
						else { break; }
					}

					// We've found a better length!
					if (bestlength as u16) < (currentlength as u16) {
						// Update the sublength slice, if provided. Note that
						// sublengths are (ZOPFLI_MAX_MATCH+1) if provided, and
						// ZOPFLI_MAX_MATCH is the largest possible value of
						// currentlength.
						if let Some(s) = sublen {
							s[bestlength as usize + 1..=currentlength as usize].fill(dist as u16);
						}

						bestdist = dist;
						bestlength = currentlength;

						// We can stop looking if we've reached the limit.
						if (currentlength as u16) >= (limit as u16) { break; }
					}
				}
			}

			// If the second chain is looking better than the first — and we
			// haven't already switched — switch to it!
			if
				! switched &&
				same0 <= (bestlength as u16) &&
				self.chain2.idx_hash[p] == self.chain2.val
			{
				switched = true;
				chain = &self.chain2;
			}

			// If there's no next previous match, we're done!
			if chain.idx_prev[p] < 0 { break; }

			// Otherwise shift to the next (previous) value.
			pp = p;
			p = chain.idx_prev[p] as usize;

			// Increase the distance accordingly.
			dist += ppp_distance(p, pp);

			// And increase the short-circuiting hits counter to prevent
			// endless work.
			hits += 1;
		} // Thus concludes the long-ass loop!

		// Return the distance and length values.
		if (bestlength as u16) <= (limit as u16) { (bestdist as u16, bestlength) }
		else { (0, LitLen::L001) }
	}
}



#[derive(Clone, Copy)]
/// # Zopfli Hash Chain.
///
/// This struct stores all recorded hash values and their latest and previous
/// positions.
///
/// Written values are all in the range of `0..=i16::MAX`, matching the array
/// sizes, elminating bounds checking on the upper end. (They're effectively
/// `u15`.)
///
/// The remaining "sign" bit is logically repurposed to serve as a sort of
/// `None` flag, allowing us to cheaply identify uninitialized values.
/// (And by testing for that, we eliminate bounds checks on the lower end of
/// the range.)
struct ZopfliHashChain {
	/// Hash value to (most recent) index.
	///
	/// Note: the original (head/head2) `hash.c` implementation was
	/// over-allocated for some reason; the hash values are masked like
	/// everything else so can't exceed `0..ZOPFLI_WINDOW_SIZE`.
	hash_idx: [i16; ZOPFLI_WINDOW_SIZE],

	/// Index to hash value (if any); this is the reverse of `hash_idx`.
	idx_hash: [i16; ZOPFLI_WINDOW_SIZE],

	/// Index to the previous index with the same hash.
	idx_prev: [i16; ZOPFLI_WINDOW_SIZE],

	/// Current hash value.
	///
	/// Note: this value defaults to zero and is never negative, but its
	/// upper range is `i16::MAX`, so the signed type still makes sense.
	val: i16,
}

impl ZopfliHashChain {
	/// # Reset.
	///
	/// (Re)Set all the data to its logical defaults so we can begin again.
	fn reset(&mut self) {
		self.hash_idx.fill(-1);
		self.idx_hash.fill(-1);
		self.idx_prev.fill(-1);
		self.val = 0;
	}

	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
		clippy::similar_names,
	)]
	/// # Update Hash.
	///
	/// This updates the index-related data for `pos`. (The hash value will
	/// have already been cycled by the time this is called.)
	fn update_hash(&mut self, pos: usize) {
		let hpos = pos & ZOPFLI_WINDOW_MASK;
		let hval = i16::max(0, self.val);

		// Update the hash.
		self.idx_hash[hpos] = hval;

		// Update the tail.
		let hash_idx = self.hash_idx[hval as usize];
		self.idx_prev[hpos] =
			if 0 <= hash_idx && self.idx_hash[hash_idx as usize] == hval {
				hash_idx
			}
			else { hpos as i16 };

		// Update the head.
		self.hash_idx[hval as usize] = hpos as i16;
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_fixed_cost() {
		// Get the largest dbit and lbit values.
		let d_max: u8 = super::super::DISTANCE_BITS.into_iter().max().unwrap();
		let l_max: u8 = super::super::LENGTH_SYMBOL_BITS.into_iter().max().unwrap();

		// Make sure their sum (along with the largest base) fits within
		// the u8 space, since that's what we're using at runtime.
		let max = u16::from(d_max) + u16::from(l_max) + 13;
		assert!(
			max <= 255,
			"maximum get_fixed_cost() is too big for u8: {max}"
		);

		// The original base is calculated by checking (279 < symbol), but we
		// instead test (114 < litlen) because the symbol isn't otherwise
		// needed. This sanity check makes sure the two conditions are indeed
		// interchangeable.
		for (i, sym) in LENGTH_SYMBOLS.iter().copied().enumerate() {
			assert_eq!(
				279 < (sym as u16),
				114 < i,
				"get_fixed_cost() base logic is wrong: len {i} has symbol {}", sym as u16
			);
		}
	}
}
