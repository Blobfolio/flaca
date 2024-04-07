/*!
# Flaca: Zopflipng Longest Match Hash.

This replaces the original `hash.c` content.
*/

use std::{
	alloc::{
		alloc,
		handle_alloc_error,
		Layout,
	},
	cell::RefCell,
	ptr::{
		addr_of,
		addr_of_mut,
		NonNull,
	},
};
use super::{
	BlockType,
	CACHE,
	calculate_block_size,
	SQUEEZE,
	LZ77Store,
	stats::{
		RanState,
		SymbolStats,
	},
	SUBLEN_LEN,
	ZOPFLI_MAX_MATCH,
	ZOPFLI_MIN_MATCH,
	ZopfliCopyLZ77Store,
	ZopfliLZ77Store,
	ZopfliStoreLitLenDist,
};

const ZOPFLI_WINDOW_SIZE: usize = 32_768;
const ZOPFLI_WINDOW_MASK: usize = ZOPFLI_WINDOW_SIZE - 1;
const HASH_SHIFT: i32 = 5;
const HASH_MASK: i16 = 32_767;
const ZOPFLI_MAX_CHAIN_HITS: usize = 8192;

/// # Length Symbols and Extra Bits.
const LENGTH_SYMBOLS_BITS: [(u16, u16); SUBLEN_LEN] = [
	(0, 0), (0, 0), (0, 0),
	(257, 0), (258, 0), (259, 0), (260, 0), (261, 0), (262, 0), (263, 0), (264, 0),
	(265, 1), (265, 1), (266, 1), (266, 1), (267, 1), (267, 1), (268, 1), (268, 1),
	(269, 2), (269, 2), (269, 2), (269, 2), (270, 2), (270, 2), (270, 2), (270, 2),
	(271, 2), (271, 2), (271, 2), (271, 2), (272, 2), (272, 2), (272, 2), (272, 2),
	(273, 3), (273, 3), (273, 3), (273, 3), (273, 3), (273, 3), (273, 3), (273, 3),
	(274, 3), (274, 3), (274, 3), (274, 3), (274, 3), (274, 3), (274, 3), (274, 3),
	(275, 3), (275, 3), (275, 3), (275, 3), (275, 3), (275, 3), (275, 3), (275, 3),
	(276, 3), (276, 3), (276, 3), (276, 3), (276, 3), (276, 3), (276, 3), (276, 3),
	(277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4),
	(277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4), (277, 4),
	(278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4),
	(278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4), (278, 4),
	(279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4),
	(279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4), (279, 4),
	(280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4),
	(280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4), (280, 4),
	(281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5),
	(281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5),
	(281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5),
	(281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5), (281, 5),
	(282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5),
	(282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5),
	(282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5),
	(282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5), (282, 5),
	(283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5),
	(283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5),
	(283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5),
	(283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5), (283, 5),
	(284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5),
	(284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5),
	(284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5),
	(284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (284, 5), (285, 0),
];

/// # Distance Bits (for minimum cost).
const MIN_COST_DISTANCES: [u8; 30] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3,
	4, 4, 5, 5, 6, 6, 7, 7, 8, 8,
	9, 9, 10, 10, 11, 11, 12, 12, 13, 13,
];



thread_local!(
	/// # Static Hash.
	///
	/// There is only ever one instance of the hash active per thread, so we
	/// might as well persist it to save on the allocations!
	pub(super) static HASH: RefCell<Box<ZopfliHash>> = RefCell::new(ZopfliHash::new());
);



#[allow(unsafe_code)]
/// # Optimal LZ77.
///
/// Calculate lit/len and dist pairs for the dataset.
///
/// Note: this incorporates the functionality of `ZopfliLZ77OptimalRun`
/// directly.
///
/// This is a rewrite of the original `squeeze.c` method.
pub(crate) fn lz77_optimal(
	arr: *const u8,
	instart: usize,
	inend: usize,
	numiterations: i32,
	store: &mut ZopfliLZ77Store,
) {
	// Easy abort.
	if instart >= inend { return; }

	// Set up and initialize a temporary LZ77 store.
	let mut current_store = LZ77Store::new(arr);

	// Reset the main cache for the current blocksize.
	let blocksize = inend - instart;
	CACHE.with_borrow_mut(|c| c.init(blocksize));

	// Initialize costs and lengths.
	SQUEEZE.with_borrow_mut(|s| {
		// And the squeeze cache!
		s.init(blocksize + 1);

		HASH.with_borrow_mut(|h| {
			// Greedy run.
			s.reset_costs();
			h.greedy(
				arr,
				instart,
				inend,
				&mut current_store,
				Some(instart),
			);

			// Create new stats with the store (updated by the greedy pass).
			let mut current_stats = SymbolStats::new();
			current_stats.load_store(&current_store);

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
				current_store.reset(arr);

				// Optimal run.
				s.reset_costs();
				let tmp = h.get_best_lengths(
					arr,
					instart,
					inend,
					Some(&current_stats),
					s.costs.as_mut_slice(),
				);
				debug_assert!(tmp < 1E30);
				s.trace_paths();
				h.follow_paths(
					arr,
					instart,
					inend,
					s.paths.as_slice(),
					&mut current_store,
				);

				// This is the cost we actually care about.
				let current_cost = calculate_block_size(
					&current_store,
					0,
					current_store.size,
					BlockType::Dynamic,
				);

				// We have a new best!
				if current_cost < best_cost {
					unsafe { ZopfliCopyLZ77Store(&*current_store, store); }
					best_stats = current_stats;
					best_cost = current_cost;
				}

				// Copy the stats to last_stats, clear them, and repopulate
				// with the current store.
				let (last_litlens, last_dists) = current_stats.clear();
				current_stats.load_store(&current_store);

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
		});
	});
}



#[derive(Clone, Copy)]
/// # Zopfli Hash.
///
/// This is a rewrite of the original `hash.c` struct.
///
/// The head/head2, prev/prev2, etc., pairs have been abstracted into their
/// own sub-structure for cleaner access, and given more meaningful names.
pub(crate) struct ZopfliHash {
	chain1: ZopfliHashChain,
	chain2: ZopfliHashChain,

	/// Repetitions of the same byte after this.
	same: [u16; ZOPFLI_WINDOW_SIZE],
}

impl ZopfliHash {
	#[allow(unsafe_code)]
	/// # New (Boxed) Instance.
	///
	/// Boxing is necessary to maintain a consistent (inner) pointer address
	/// for the main object, and to store the arrays on the heap rather than
	/// the stack.
	///
	/// Credit to the zopfli-rs port for laying the groundwork!
	///
	/// ## Safety.
	///
	/// This allocates the struct *without* initializing it; `Self::init` must
	/// be called before it can actually be used.
	fn new() -> Box<Self> {
		const LAYOUT: Layout = Layout::new::<ZopfliHash>();

		unsafe {
			NonNull::new(alloc(LAYOUT).cast())
				.map_or_else(
					|| handle_alloc_error(LAYOUT),
					|ptr| Box::from_raw(ptr.as_ptr())
				)
		}
	}

	#[allow(unsafe_code)]
	/// # Initialize Values.
	///
	/// Initialize/reset hash values to their defaults so we can reuse the
	/// structure for a new dataset.
	unsafe fn init(&mut self) {
		// All the hash/index arrays default to `-1` for `None`; thanks to
		// Rust's complementary notation, we can achieve this quickly by
		// flipping on all the bits.
		addr_of_mut!(self.chain1.hash_idx).write_bytes(u8::MAX, 1);
		addr_of_mut!(self.chain1.idx_hash).write_bytes(u8::MAX, 1);
		addr_of_mut!(self.chain1.idx_prev).write_bytes(u8::MAX, 1);

		// The initial hash value is just plain zero.
		addr_of_mut!(self.chain1.val).write(0);

		// The second chain is the same as the first, so we can simply copy it
		// wholesale.
		addr_of_mut!(self.chain2).copy_from_nonoverlapping(addr_of!(self.chain1), 1);

		// Repetitions default to zero; thanks to zero being zeros all the way
		// down, we can achieve this by flipping off all the bits.
		addr_of_mut!(self.same).write_bytes(0, 1);
	}

	#[allow(unsafe_code)]
	/// # Reset/Warm Up.
	fn reset(
		&mut self,
		arr: *const u8,
		instart: usize,
		inend: usize,
	) {
		let windowstart = instart.saturating_sub(ZOPFLI_WINDOW_SIZE);
		unsafe {
			// Set all values to their defaults.
			self.init();

			// Cycle the hash once or twice.
			self.update_hash_value(*arr.add(windowstart));
			if windowstart + 1 < inend {
				self.update_hash_value(*arr.add(windowstart + 1));
			}
		}

		// Process the values between windowstart and instart.
		for i in windowstart..instart {
			self.update_hash(
				unsafe { std::slice::from_raw_parts(arr.add(i), inend - i) },
				i,
			);
		}
	}

	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::similar_names,
	)]
	#[inline]
	/// # Update Hash.
	///
	/// Note that unlike the original method, `arr` is pre-sliced to the
	/// relevant region.
	fn update_hash(&mut self, arr: &[u8], pos: usize) {
		let hpos = pos & ZOPFLI_WINDOW_MASK;

		// Cycle the first hash.
		self.update_hash_value(arr.get(ZOPFLI_MIN_MATCH - 1).map_or(0, |v| *v));
		self.chain1.update_hash(pos);

		// Count up the repetitions (and update sameness).
		let mut amount = self.same[pos.wrapping_sub(1) & ZOPFLI_WINDOW_MASK]
			.saturating_sub(1);
		while
			amount < u16::MAX &&
			usize::from(amount) + 1 < arr.len() &&
			arr[0] == arr[usize::from(amount) + 1]
		{
			amount += 1;
		}
		self.same[hpos] = amount;

		// Cycle the second hash.
		self.chain2.val = (((amount - ZOPFLI_MIN_MATCH as u16) & 255) as i16) ^ self.chain1.val;
		self.chain2.update_hash(pos);
	}

	/// # Update Hash Value.
	///
	/// This updates the rotating (chain1) value.
	fn update_hash_value(&mut self, c: u8) {
		self.chain1.val = ((self.chain1.val << HASH_SHIFT) ^ i16::from(c)) & HASH_MASK;
	}
}

impl ZopfliHash {
	#[allow(
		unsafe_code,
		clippy::cast_possible_truncation,
	)]
	/// # Get Best Lengths.
	///
	/// This method performs the forward pass for "squeeze", calculating the
	/// optimal length to reach every byte from a previous byte. The resulting
	/// cost is returned.
	///
	/// Note: the repeated float truncation looks like an oversight but is
	/// intentional; trying to use only one or the other exclusively alters the
	/// outcome, so whatever. Haha.
	///
	/// This is a rewrite of the original `squeeze.c` method.
	pub(super) fn get_best_lengths(
		&mut self,
		arr: *const u8,
		instart: usize,
		inend: usize,
		stats: Option<&SymbolStats>,
		costs: &mut [(f32, u16)],
	) -> f64 {
		// Costs and lengths are resized prior to this point; they should be
		// one larger than the data of interest (and equal to each other).
		debug_assert!(costs.len() == inend - instart + 1);

		// Reset and warm the hash.
		self.reset(arr, instart, inend);

		let mut length = 0_u16;
		let mut distance = 0_u16;
		let mut sublen = [0_u16; SUBLEN_LEN];

		// Find the minimum and maximum cost.
		let min_cost = stats.map_or(12.0, get_minimum_cost);

		// Convert the array to a slice for safer reslicing.
		let arr = unsafe { std::slice::from_raw_parts(arr, inend) };
		let mut i = instart;
		while i < arr.len() {
			// Hash the remainder.
			self.update_hash(&arr[i..], i);

			// Relative position for the costs and lengths, which have
			// (iend - istart + 1) entries, so j is always in range when i is.
			let mut j = i - instart;

			// We're in a long repetition of the same character and have more
			// than ZOPFLI_MAX_MATCH ahead of and behind us.
			if self._get_best_lengths_max_match(instart, i, stats, arr, costs) {
				i += ZOPFLI_MAX_MATCH;
				j += ZOPFLI_MAX_MATCH;
			}

			// Find the longest remaining match.
			self.find(
				arr.as_ptr(),
				i,
				arr.len(),
				ZOPFLI_MAX_MATCH,
				&mut sublen,
				&mut distance,
				&mut length,
				Some(instart),
			);

			// Literal. (Note i is always < arr.len() here, but checking again
			// makes the compiler happy, so whatever.)
			let cost_j = f64::from(unsafe { costs.get_unchecked(j).0 });
			if i < arr.len() && j + 1 < costs.len() {
				let new_cost = stats.map_or(
					if arr[i] <= 143 { 8.0 } else { 9.0 },
					|s| s.ll_symbols[usize::from(arr[i])],
				) + cost_j;
				debug_assert!(0.0 <= new_cost);

				// Update it if lower.
				if new_cost < f64::from(costs[j + 1].0) {
					costs[j + 1].0 = new_cost as f32;
					costs[j + 1].1 = 1;
				}
			}

			// Lengths and Sublengths.
			let limit = usize::from(length).min(arr.len() - i);
			if (ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH).contains(&limit) {
				let min_cost_add = min_cost + cost_j;
				let mut k = ZOPFLI_MIN_MATCH;
				for (&v, c) in sublen[k..=limit].iter().zip(costs.iter_mut().skip(j + k)) {
					// The expensive cost calculations are only worth
					// performing if the stored cost is larger than the
					// minimum cost we found earlier.
					if min_cost_add < f64::from(c.0) {
						let new_cost = stats.map_or_else(
							|| get_fixed_cost(k as u16, v),
							|s| get_stat_cost(k as u16, v, s),
						) + cost_j;
						debug_assert!(0.0 <= new_cost);

						// Update it if lower.
						if new_cost < f64::from(c.0) {
							c.0 = new_cost as f32;
							c.1 = k as u16;
						}
					}
					k += 1;
				}
			}

			// Back around again!
			i += 1;
		}

		// Return the final cost!
		debug_assert!(0.0 <= costs[costs.len() - 1].0);
		f64::from(costs[costs.len() - 1].0)
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Best Length Max Match.
	///
	/// This fast-forwards through long repetitions in the middle of a
	/// `ZopfliHash::get_best_lengths` block, processing `ZOPFLI_MAX_MATCHES`
	/// `arr` and `costs` entries in one go.
	///
	/// Returns `true` if such a match was found so the indices can be
	/// incremented accordingly on the caller's side.
	fn _get_best_lengths_max_match(
		&mut self,
		instart: usize,
		mut pos: usize,
		stats: Option<&SymbolStats>,
		arr: &[u8],
		costs: &mut [(f32, u16)],
	) -> bool {
		if
			// We have more than ZOPFLI_MAX_MATCH entries behind us, and twice
			// twice as many ahead of us.
			pos > instart + ZOPFLI_MAX_MATCH + 1 &&
			arr.len() > pos + ZOPFLI_MAX_MATCH * 2 + 1 &&
			// The current and max-match-ago positions have long repetitions.
			self.same[pos & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16 * 2 &&
			self.same[(pos - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16
		{
			// The symbol cost for ZOPFLI_MAX_LENGTH (and a distance of 1) doesn't
			// need mutch calculation.
			let symbol_cost = stats.map_or(
				13.0,
				|s| (s.ll_symbols[285] + s.d_symbols[0]),
			);

			// Costs are sized to (inend - instart + 1) so anytime arr is in
			// range, so is costs, but the compiler doesn't understand that.
			// Recasting to a fixed array upfront helps it understand we have
			// sufficient entries for the loop.
			let costs: &mut [(f32, u16); ZOPFLI_MAX_MATCH * 2] = unsafe {
				&mut *costs.as_mut_ptr().add(pos - instart).cast()
			};

			// Fast forward!
			for j in 0..ZOPFLI_MAX_MATCH {
				costs[j + ZOPFLI_MAX_MATCH].0 = (f64::from(costs[j].0) + symbol_cost) as f32;
				costs[j + ZOPFLI_MAX_MATCH].1 = ZOPFLI_MAX_MATCH as u16;
				pos += 1;
				self.update_hash(&arr[pos..], pos);
			}

			true
		}
		else { false }
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Greedy LZ77 Run.
	pub(crate) fn greedy(
		&mut self,
		arr: *const u8,
		instart: usize,
		inend: usize,
		store: &mut ZopfliLZ77Store,
		cache: Option<usize>,
	) {
		// Reset the hash.
		self.reset(arr, instart, inend);

		// Convert the input to a proper slice.
		let arr = unsafe { std::slice::from_raw_parts(arr, inend) };

		// We'll need a few more variables…
		let mut sublen = [0_u16; SUBLEN_LEN];
		let mut length: u16 = 0;
		let mut distance: u16 = 0;
		let mut prev_length: u16 = 0;
		let mut prev_distance: u16 = 0;
		let mut match_available = false;

		// Loop the data!
		let mut i = instart;
		while i < arr.len() {
			// Update the hash.
			self.update_hash(&arr[i..], i);

			// Run the finder.
			self.find(
				arr.as_ptr(),
				i,
				arr.len(),
				ZOPFLI_MAX_MATCH,
				&mut sublen,
				&mut distance,
				&mut length,
				cache,
			);

			// Lazy matching.
			let length_score = get_length_score(length, distance);
			let prev_length_score = get_length_score(prev_length, prev_distance);
			if match_available {
				match_available = false;

				if length_score > prev_length_score + 1 {
					unsafe {
						ZopfliStoreLitLenDist(
							// This isn't accessible on the first iteration so
							// -1 is always in range.
							u16::from(*arr.get_unchecked(i - 1)),
							0,
							i - 1,
							store,
						);
					}
					if length_score >= ZOPFLI_MIN_MATCH as u16 && length < ZOPFLI_MAX_MATCH as u16 {
						match_available = true;
						prev_length = length;
						prev_distance = distance;

						i += 1;
						continue;
					}
				}
				else {
					// Old is new.
					length = prev_length;
					distance = prev_distance;

					// Write the values!
					unsafe { ZopfliStoreLitLenDist(length, distance, i - 1, store); }

					// Update the hash up through length and increment the loop
					// position accordingly.
					for _ in 2..length {
						i += 1;
						self.update_hash(&arr[i..], i);
					}

					i += 1;
					continue;
				}
			}
			// No previous match, but maybe we can set it for the next
			// iteration?
			else if length_score >= ZOPFLI_MIN_MATCH as u16 && length < ZOPFLI_MAX_MATCH as u16 {
				match_available = true;
				prev_length = length;
				prev_distance = distance;

				i += 1;
				continue;
			}

			// Write the current length/distance.
			if length_score >= ZOPFLI_MIN_MATCH as u16 {
				unsafe { ZopfliStoreLitLenDist(length, distance, i, store); }
			}
			// Write from the source with no distance and reset the length to
			// one.
			else {
				length = 1;
				unsafe { ZopfliStoreLitLenDist(u16::from(arr[i]), 0, i, store); }
			}

			// Update the hash up through length and increment the loop
			// position accordingly.
			for _ in 1..length {
				i += 1;
				self.update_hash(&arr[i..], i);
			}

			i += 1;
		}
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Follow Paths.
	pub(super) fn follow_paths(
		&mut self,
		arr: *const u8,
		instart: usize,
		inend: usize,
		paths: &[u16],
		store: &mut ZopfliLZ77Store,
	) {
		// Easy abort.
		if instart >= inend { return; }

		// Verify all the lengths will fit so we can safely skip bounds
		// checking during iteration.
		assert!(
			instart + paths.iter().map(|&n| usize::from(n)).sum::<usize>() <= inend
		);

		// Reset the hash.
		self.reset(arr, instart, inend);

		// Hash the path symbols.
		let arr = unsafe { std::slice::from_raw_parts(arr, inend) };

		let mut i = instart;
		for &length in paths.iter().rev() {
			self.update_hash(unsafe { arr.get_unchecked(i..) }, i);

			// Add to output.
			if length >= ZOPFLI_MIN_MATCH as u16 {
				// Get the distance by recalculating the longest match, and
				// make sure the length matches afterwards (as that's easy to
				// screw up!).
				let mut test_length = 0;
				let mut dist = 0;
				self.find(
					arr.as_ptr(),
					i,
					arr.len(),
					usize::from(length),
					&mut [],
					&mut dist,
					&mut test_length,
					Some(instart),
				);
				assert!(! (test_length != length && length > 2 && test_length > 2));

				// Add it to the store.
				unsafe { ZopfliStoreLitLenDist(length, dist, i, store); }

				// Hash the rest of the match.
				for _ in 1..usize::from(length) {
					i += 1;
					self.update_hash(unsafe { arr.get_unchecked(i..) }, i);
				}
			}
			// Add it to the store.
			else {
				unsafe { ZopfliStoreLitLenDist(u16::from(arr[i]), 0, i, store); }
			}

			i += 1;
		}
	}
}

impl ZopfliHash {
	#[allow(unsafe_code, clippy::too_many_arguments)]
	/// # Find Longest Match.
	///
	/// This is a rewrite of the original `lz77.c` method.
	fn find(
		&self,
		arr: *const u8,
		pos: usize,
		size: usize,
		mut limit: usize,
		sublen: &mut [u16],
		distance: &mut u16,
		length: &mut u16,
		cache: Option<usize>,
	) {
		// Check the longest match cache first!
		if let Some(blockstart) = cache {
			if CACHE.with_borrow(|c| c.find(
				pos - blockstart,
				&mut limit,
				sublen,
				distance,
				length,
			)) {
				assert!(pos + usize::from(*length) <= size);
				return;
			}
		}

		// These are both hard-coded or asserted by the caller.
		debug_assert!((ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH).contains(&limit));
		debug_assert!(pos < size);

		// We'll need at least ZOPFLI_MIN_MATCH bytes for a search; if we don't
		// have it, zero everything out and call it a day.
		if size - pos < ZOPFLI_MIN_MATCH {
			*length = 0;
			*distance = 0;
			return;
		}

		// Cap the limit to fit if needed. Note that limit will always be at
		// least one even if capped since pos < size.
		if pos + limit > size { limit = size - pos; }

		// Calculate the best distance and length.
		let (bestdist, bestlength) = self.find_loop(arr, pos, size, limit, sublen);

		// Cache the results for next time, maybe.
		if let Some(blockstart) = cache {
			if limit == ZOPFLI_MAX_MATCH && ! sublen.is_empty() {
				CACHE.with_borrow_mut(|c|
					c.set_sublen(pos - blockstart, sublen, bestdist, bestlength)
				);
			}
		}

		// Update the values.
		*distance = bestdist;
		*length = bestlength;
		assert!(pos + usize::from(*length) <= size);
	}

	#[allow(
		unsafe_code,
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_ptr_alignment,
		clippy::cast_sign_loss,
		clippy::similar_names,
	)]
	/// # Find Longest Match Loop.
	///
	/// This method is the (nasty-looking) workhorse of the above
	/// `ZopfliCache::find` method. It finds and returns the matching distance
	/// and length, or `(0, 1)` if none.
	fn find_loop(
		&self,
		arr: *const u8,
		pos: usize,
		size: usize,
		limit: usize,
		sublen: &mut [u16],
	) -> (u16, u16) {
		let hpos = pos & ZOPFLI_WINDOW_MASK;

		// The default distance and length. Note we're using usize here (and
		// elsewhere) to help minimize typecasting for comparisons,
		// assignments, etc.
		let mut bestdist: usize = 0;
		let mut bestlength: usize = 1;

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
		let mut dist =
			if p < pp { pp - p }
			else { ZOPFLI_WINDOW_SIZE + pp - p };

		let mut hits = 0;
		let same0 = usize::from(self.same[hpos]);
		let same1 = usize::min(same0, limit);
		while p < ZOPFLI_WINDOW_SIZE && dist < ZOPFLI_WINDOW_SIZE && hits < ZOPFLI_MAX_CHAIN_HITS {
			let mut currentlength = 0;

			// These are simple sanity assertions; the values are only ever
			// altered via ZopfliHashChain::update_hash so there isn't much
			// room for mistake.
			debug_assert!(p as i16 == chain.idx_prev[pp] || p == pp);
			debug_assert_eq!(chain.idx_hash[p], chain.val);

			// If we have distance, we can look for matches!
			if 0 < dist && dist <= pos {
				// Note: this logic is too convoluted for the Rust compiler
				// so it is significantly more performant to work from
				// pointers. The main things to note are:
				// * (match_idx <= scan_idx) throughout
				// * (limit <= ZOPFLI_MAX_MATCHES)
				// * (pos + limit <= arr.len())
				// * (same <= limit), so (pos + same <= arr.len()) too
				// * best/currentlength is always between pos..=arr.len()
				let mut scan_idx = pos;
				let mut match_idx = pos - dist;

				// If the scan and match indexes hold the same value, peek
				// ahead to find the length of the match.
				if
					pos + bestlength >= size ||
					unsafe { *arr.add(scan_idx + bestlength) == *arr.add(match_idx + bestlength)}
				{
					if 2 < same0 && unsafe { *arr.add(scan_idx) == *arr.add(match_idx) } {
						let same2 = usize::from(self.same[match_idx & ZOPFLI_WINDOW_MASK]);
						let same = usize::min(same1, same2);
						scan_idx += same;
						match_idx += same;
					}

					// Look for additional matches up to the limit (and within
					// the bounds of arr), eight bytes at a time since PNG data
					// errs on the repetitive side.
					while scan_idx + 8 < pos + limit && unsafe { *arr.add(scan_idx).cast::<u64>() == *arr.add(match_idx).cast::<u64>() } {
						scan_idx += 8;
						match_idx += 8;
					}

					// And do the same for any remaining bytes, individually.
					while scan_idx < pos + limit && unsafe { *arr.add(scan_idx) == *arr.add(match_idx) } {
						scan_idx += 1;
						match_idx += 1;
					}

					// The length is the distance scan_idx has traveled.
					currentlength = scan_idx - pos;
				}

				// We've found a better length!
				if bestlength < currentlength {
					// Update the sublength slice, if provided.
					if ! sublen.is_empty() {
						// Safety: this is represented as a generic slice only
						// because [u16; ZOPFLI_MAX_MATCHES + 1] isn't copy.
						// The best/currentlength values are capped to limit
						// which is capped to ZOPFLI_MAX_MATCHES, so there'll
						// always be room.
						unsafe {
							sublen.get_unchecked_mut(bestlength + 1..=currentlength).fill(dist as u16);
						}
					}

					bestdist = dist;
					bestlength = currentlength;

					// We can stop looking if we've reached the limit.
					if currentlength >= limit { break; }
				}
			}

			// If the second chain is looking better than the first — and we
			// haven't already switched — switch to it!
			if
				! switched &&
				same0 <= bestlength &&
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
			dist +=
				if p < pp { pp - p }
				else { ZOPFLI_WINDOW_SIZE + pp - p };

			// And increase the short-circuiting hits counter to prevent
			// endless work.
			hits += 1;
		} // Thus concludes the long-ass loop!

		// Return the distance and length values.
		if bestlength <= limit { (bestdist as u16, bestlength as u16) }
		else { (0, 1) }
	}
}



#[derive(Clone, Copy)]
/// # Zopfli Hash Chain.
///
/// This struct stores all recorded hash values and their latest and previous
/// positions.
///
/// Written values are all in the range of `0..=i16::MAX`, matching the array
/// sizes, elminating bounds checking on the upper end.
///
/// The remaining "sign" bit is logically repurposed to serve as a sort of
/// `None`, allowing us to cheaply identify unwritten values. (Testing for that
/// takes care of bounds checking on the lower end.)
pub(crate) struct ZopfliHashChain {
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
	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
		clippy::similar_names,
	)]
	/// # Update Hash.
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



#[allow(clippy::cast_possible_truncation)]
/// # Distance Symbol and Extra Bits.
///
/// Calculate the symbol and bits given the distance. There is unfortunately
/// too much variation to justify a simple table like the one used for lengths;
/// (compiler-optimized) math is our best bet.
const fn distance_symbol_bits(dist: u32) -> (u16, u16) {
	if dist < 5 { (dist as u16 - 1, 0) }
	else {
		let d_log = (dist - 1).ilog2();
		let r = ((dist - 1) >> (d_log - 1)) & 1;
		let sym = (d_log * 2 + r) as u16;
		(sym, (d_log - 1) as u16)
	}
}

#[allow(
	unsafe_code,
	clippy::cast_possible_truncation,
	clippy::similar_names,
)]
/// # Fixed Cost Model.
///
/// This models the cost using a fixed tree.
fn get_fixed_cost(len: u16, dist: u16) -> f64 {
	if dist == 0 {
		if len <= 143 { 8.0 }
		else { 9.0 }
	}
	else {
		let (lsym, lbits) = unsafe {
			// Safety: this is only ever called with lengths between MIN..=MAX
			// so values are always in range.
			*LENGTH_SYMBOLS_BITS.get_unchecked(usize::from(len))
		};
		let dbits =
			if dist < 5 { 0 }
			else { (dist - 1).ilog2() as u16 - 1 };
		let base =
			if 279 < lsym { 13 }
			else { 12 };

		f64::from(base + dbits + lbits)
	}
}

/// # Distance-Based Length Score.
///
/// This is a simplistic cost model for the "greedy" LZ77 pass that helps it
/// make a slightly better choice between two options during lazy matching.
///
/// This is a rewrite of the original `lz77.c` method.
const fn get_length_score(length: u16, distance: u16) -> u16 {
	if 1024 < distance { length - 1 }
	else { length }
}

#[allow(
	unsafe_code,
	clippy::cast_possible_truncation,
	clippy::similar_names,
)]
/// # Minimum Cost Model.
///
/// This returns the minimum _statistical_ cost, which is the sum of the
/// minimum length cost and minimum distance cost.
fn get_minimum_cost(stats: &SymbolStats) -> f64 {
	// Find the minimum length cost.
	let mut length_cost = f64::INFINITY;
	for &(lsym, lbits) in LENGTH_SYMBOLS_BITS.iter().skip(3) {
		// Safety: the largest length symbol is 285; the last index of
		// ll_symbols is 287.
		let cost = f64::from(lbits) + unsafe { *stats.ll_symbols.get_unchecked(lsym as usize) };
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

#[allow(
	unsafe_code,
	clippy::cast_possible_truncation,
	clippy::similar_names,
)]
/// # Statistical Cost Model.
///
/// This models the cost using the gathered symbol statistics.
fn get_stat_cost(len: u16, dist: u16, stats: &SymbolStats) -> f64 {
	if dist == 0 {
		// Safety: this is only ever called with lengths between MIN..=MAX so
		// values are always in range.
		unsafe { *stats.ll_symbols.get_unchecked(usize::from(len)) }
	}
	else {
		// Safety: this is only ever called with lengths between MIN..=MAX so
		// values are always in range.
		let (lsym, lbits) = unsafe {
			*LENGTH_SYMBOLS_BITS.get_unchecked(usize::from(len))
		};
		let (dsym, dbits) = distance_symbol_bits(u32::from(dist));

		f64::from(lbits + dbits) +
		unsafe {
			// Safety: all returned symbols are in range.
			*stats.ll_symbols.get_unchecked(lsym as usize) +
			*stats.d_symbols.get_unchecked(dsym as usize)
		}
	}
}
