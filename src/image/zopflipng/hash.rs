/*!
# Flaca: Zopflipng Matches and Hashes.

This module contains the zopfli match-hashing functionality, including the
thread-local `HASH` static.
*/

use std::{
	alloc::{
		alloc,
		handle_alloc_error,
		Layout,
	},
	cell::{
		Cell,
		RefCell,
	},
	ptr::{
		addr_of,
		addr_of_mut,
		NonNull,
	},
};
use super::{
	CACHE,
	DISTANCE_BITS,
	DISTANCE_SYMBOLS,
	LENGTH_SYMBOLS_BITS_VALUES,
	LZ77Store,
	SqueezeCache,
	stats::SymbolStats,
	SUBLEN_LEN,
	ZOPFLI_MAX_MATCH,
	ZOPFLI_MIN_MATCH,
	ZopfliError,
};

const ZOPFLI_WINDOW_SIZE: usize = 32_768;
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



thread_local!(
	/// # Static Hash.
	///
	/// There is only ever one instance of the hash active per thread, so we
	/// might as well persist it to save on the allocations!
	pub(super) static HASH: RefCell<Box<ZopfliHash>> = RefCell::new(ZopfliHash::new());
);



#[derive(Clone, Copy)]
/// # Zopfli Hash.
///
/// This structure tracks byte values and hashes by position, facilitating
/// match-finding (length and distance) at various offsets.
///
/// It is functionally equivalent to the original `hash.c` structure, but with
/// more consistent member typing, sizing, and naming.
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
	/// Fixed arrays really do seem to be the most efficient structure for
	/// this data — even though `HashMap` seems ready-made for the job! — but
	/// they're way too big to throw on the stack willynilly.
	///
	/// Taking a page from the [`zopfli-rs`](https://github.com/zopfli-rs/zopfli)
	/// port, new instances are initialized from raw pointers and `Box`ed to
	/// keep them on the heap.
	///
	/// ## Safety.
	///
	/// The return value is allocated but **uninitialized**. Re/initialization
	/// occurs subsequently when `ZopfliHash::reset` is called.
	///
	/// There are only two entrypoints into the thread-local static holding
	/// this data — `ZopfliHash::greedy` and `ZopfliHash::optimal_run` — and
	/// both reset as their _first order of business_, so in practice
	/// everything is A-OK!
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
		// All the hash/index arrays default to `-1_i16` for `None`, which we
		// can do efficiently by setting all bits to one.
		addr_of_mut!(self.chain1.hash_idx).write_bytes(u8::MAX, 1);
		addr_of_mut!(self.chain1.idx_hash).write_bytes(u8::MAX, 1);
		addr_of_mut!(self.chain1.idx_prev).write_bytes(u8::MAX, 1);

		// The initial hash value is just plain zero.
		addr_of_mut!(self.chain1.val).write(0);

		// The second chain is the same as the first, so we can simply copy it
		// wholesale.
		addr_of_mut!(self.chain2).copy_from_nonoverlapping(addr_of!(self.chain1), 1);

		// Repetitions default to zero.
		addr_of_mut!(self.same).write_bytes(0, 1);
	}

	#[allow(unsafe_code)]
	/// # Reset/Warm Up.
	///
	/// This sets all values to their defaults, then cycles the first chain's
	/// hash value once or twice, then hashes the bits between the start of the
	/// window and the start of the slice we're actually interested in, if any.
	fn reset(
		&mut self,
		arr: &[u8],
		instart: usize,
	) {
		unsafe { self.init(); }

		// Cycle the hash once or twice.
		if instart >= arr.len() { return; }
		let windowstart = instart.saturating_sub(ZOPFLI_WINDOW_SIZE);
		self.update_hash_value(arr[windowstart]);
		if windowstart + 1 < arr.len() {
			self.update_hash_value(arr[windowstart + 1]);
		}

		// Process the values between windowstart and instart.
		for i in windowstart..instart { self.update_hash(&arr[i..], i); }
	}

	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::similar_names,
	)]
	#[inline]
	/// # Update Hash.
	///
	/// This updates the hash tables using the data from `arr`. The `pos` value
	/// marks the position of `arr` within the original block slice. (That is,
	/// `arr` is pre-sliced to `arr[pos..]` before being passed to this method.)
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
	/// This updates the rotating (chain1) hash value.
	fn update_hash_value(&mut self, c: u8) {
		self.chain1.val = ((self.chain1.val << HASH_SHIFT) ^ i16::from(c)) & HASH_MASK;
	}
}

impl ZopfliHash {
	/// # Optimal Run.
	///
	/// This performs backward/forward squeeze passes on the data, optionally
	/// considering existing histogram data. The `store` is updated with the
	/// best-length match data.
	///
	/// This is one of two possible entrypoints into the thread-local
	/// `ZopfliHash` instance.
	pub(crate) fn optimal_run(
		&mut self,
		arr: &[u8],
		instart: usize,
		stats: Option<&SymbolStats>,
		squeeze: &mut SqueezeCache,
		store: &mut LZ77Store,
	) -> Result<(), ZopfliError> {
		let costs = squeeze.reset_costs();
		self.get_best_lengths(arr, instart, stats, costs)?;
		if let Some(paths) = squeeze.trace_paths() {
			self.follow_paths(arr, instart, paths, store)?;
		}
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	#[inline]
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
	fn get_best_lengths(
		&mut self,
		arr: &[u8],
		instart: usize,
		stats: Option<&SymbolStats>,
		costs: &mut [(f32, u16)],
	) -> Result<(), ZopfliError> {
		// Reset and warm the hash.
		self.reset(arr, instart);

		// Costs and lengths are resized prior to this point; they should be
		// one larger than the data of interest (and equal to each other).
		debug_assert!(costs.len() == arr.len() - instart + 1);

		let mut length = 0_u16;
		let mut distance = 0_u16;
		let mut sublen = [0_u16; SUBLEN_LEN];

		// Find the minimum and maximum cost.
		let min_cost = stats.map_or(12.0, get_minimum_cost);

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
				arr,
				i,
				ZOPFLI_MAX_MATCH,
				&mut sublen,
				&mut distance,
				&mut length,
				Some(instart),
			)?;

			// This should never trigger; it is mainly a reminder to the
			// compiler that our i/j indices are still applicable.
			if i >= arr.len() || j + 1 >= costs.len() { break; }

			let cost_j = f64::from(costs[j].0);
			let new_cost = stats.map_or_else(
				|| if arr[i] <= 143 { 8.0 } else { 9.0 },
				|s| s.ll_symbols[usize::from(arr[i])],
			) + cost_j;
			debug_assert!(0.0 <= new_cost);

			// Update it if lower.
			if new_cost < f64::from(costs[j + 1].0) {
				costs[j + 1].0 = new_cost as f32;
				costs[j + 1].1 = 1;
			}

			// If a long match was found, peek forward to recalculate those
			// costs, at least the ones who could benefit from the expense of
			// all that effort.
			let limit = usize::from(length).min(costs.len().saturating_sub(j + 1));
			if (ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH).contains(&limit) {
				let min_cost_add = min_cost + cost_j;
				let mut k = ZOPFLI_MIN_MATCH;
				let iter = sublen[k..=limit].iter()
					.copied()
					.zip(costs.iter_mut().skip(j + k));

				// Stat-based cost calculations.
				if let Some(s) = stats {
					for (dist, c) in iter {
						if min_cost_add < f64::from(c.0) {
							let mut new_cost = cost_j;
							if dist == 0 {
								new_cost += s.ll_symbols[k];
							}
							else {
								let dsym = DISTANCE_SYMBOLS[(dist & 32_767) as usize];
								new_cost += f64::from(DISTANCE_BITS[dsym as usize]);
								new_cost += s.d_symbols[dsym as usize];
								new_cost += s.ll_symbols[LENGTH_SYMBOLS_BITS_VALUES[k].0 as usize];
								new_cost += f64::from(LENGTH_SYMBOLS_BITS_VALUES[k].1);
							}

							// Update it if lower.
							if (0.0..f64::from(c.0)).contains(&new_cost) {
								c.0 = new_cost as f32;
								c.1 = k as u16;
							}
						}
						k += 1;
					}
				}
				// Fixed cost calculations.
				else {
					for (dist, c) in iter {
						if min_cost_add < f64::from(c.0) {
							let mut new_cost = cost_j;
							if dist == 0 {
								if k <= 143 { new_cost += 8.0; }
								else { new_cost += 9.0; }
							}
							else {
								if 114 < k { new_cost += 13.0; }
								else { new_cost += 12.0; }

								let dsym = DISTANCE_SYMBOLS[(dist & 32_767) as usize];
								new_cost += f64::from(DISTANCE_BITS[dsym as usize]);
								new_cost += f64::from(LENGTH_SYMBOLS_BITS_VALUES[k].1);
							}

							// Update it if lower.
							if (0.0..f64::from(c.0)).contains(&new_cost) {
								c.0 = new_cost as f32;
								c.1 = k as u16;
							}
						}
						k += 1;
					}
				}
			}

			// Back around again!
			i += 1;
		}

		// The final cost should in a reasonable range.
		debug_assert!((0.0..1E30).contains(&costs[costs.len() - 1].0));
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	#[inline]
	/// # Best Length Max Match.
	///
	/// This fast-forwards through long repetitions in the middle of a
	/// `ZopfliHash::get_best_lengths` block, processing `ZOPFLI_MAX_MATCH`
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

			// We'll need to read data from one portion of the slice and add it
			// to data in another portion. Index-based access confusing the
			// compiler, so to avoid a bunch of "unsafe", we'll work with a
			// slice-of-cells representation instead.
			let costs = Cell::from_mut(costs).as_slice_of_cells();

			// Fast forward!
			let before = pos;
			let mut iter = costs.windows(ZOPFLI_MAX_MATCH + 1).skip(pos - instart).take(ZOPFLI_MAX_MATCH);
			while let Some([a, _rest @ .., z]) = iter.next() {
				z.set((
					(f64::from(a.get().0) + symbol_cost) as f32,
					ZOPFLI_MAX_MATCH as u16,
				));
				pos += 1;
				self.update_hash(&arr[pos..], pos);
			}

			// We should never not hit our desired take() because the lengths
			// of arr and cost are fixed and intertwined, but it's a good debug
			// sort of thing to check.
			debug_assert_eq!(pos - before, ZOPFLI_MAX_MATCH);

			true
		}
		else { false }
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Greedy LZ77 Run.
	///
	/// This method looks for best-length matches in the data (and/or cache),
	/// updating the store with the results.
	///
	/// This is one of two entrypoints into the thread-local `ZopfliHash`
	/// instance.
	pub(crate) fn greedy(
		&mut self,
		arr: &[u8],
		instart: usize,
		store: &mut LZ77Store,
		cache: Option<usize>,
	) -> Result<(), ZopfliError> {
		// Reset the hash.
		self.reset(arr, instart);

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
				arr,
				i,
				ZOPFLI_MAX_MATCH,
				&mut sublen,
				&mut distance,
				&mut length,
				cache,
			)?;

			// Lazy matching.
			let length_score = get_length_score(length, distance);
			let prev_length_score = get_length_score(prev_length, prev_distance);
			if match_available {
				match_available = false;

				if length_score > prev_length_score + 1 {
					// Safety: match_available starts false so even if instart
					// is zero, we won't reach this part until we've iterated
					// at least once.
					store.push(
						u16::from(unsafe { *arr.get_unchecked(i - 1) }),
						0,
						i - 1,
					)?;
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
					store.push(length, distance, i - 1)?;

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
				store.push(length, distance, i)?;
			}
			// Write from the source with no distance and reset the length to
			// one.
			else {
				length = 1;
				store.push(u16::from(arr[i]), 0, i)?;
			}

			// Update the hash up through length and increment the loop
			// position accordingly.
			for _ in 1..length {
				i += 1;
				self.update_hash(&arr[i..], i);
			}

			i += 1;
		}

		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Follow Paths.
	///
	/// This method repopulates the hash tables by following the provided
	/// squeeze-based path lengths. The store is updated with the results.
	fn follow_paths(
		&mut self,
		arr: &[u8],
		instart: usize,
		paths: &[u16],
		store: &mut LZ77Store,
	) -> Result<(), ZopfliError> {
		// Easy abort.
		if instart >= arr.len() { return Ok(()); }

		// Reset the hash.
		self.reset(arr, instart);

		// Hash the path symbols.
		let mut i = instart;
		for &length in paths.iter().rev() {
			self.update_hash(&arr[i..], i);

			// Add to output.
			if length >= ZOPFLI_MIN_MATCH as u16 {
				// Get the distance by recalculating the longest match, and
				// make sure the length matches afterwards (as that's easy to
				// screw up!).
				let mut test_length = 0;
				let mut dist = 0;
				self.find(
					arr,
					i,
					usize::from(length),
					&mut [],
					&mut dist,
					&mut test_length,
					Some(instart),
				)?;

				// This logic is so screwy; I hesitate to make this a debug
				// assertion!
				if test_length != length && length > 2 && test_length > 2 {
					return Err(ZopfliError::PathLength);
				}

				// Add it to the store.
				store.push(length, dist, i)?;

				// Hash the rest of the match.
				for _ in 1..usize::from(length) {
					i += 1;
					self.update_hash(&arr[i..], i);
				}
			}
			// Add it to the store.
			else {
				store.push(u16::from(arr[i]), 0, i)?;
			}

			i += 1;
		}

		Ok(())
	}
}

impl ZopfliHash {
	#[allow(clippy::too_many_arguments)]
	/// # Find Longest Match.
	///
	/// This finds the longest match in `arr` (and/or the cache), setting the
	/// passed `sublen`/`distance`/`length` values accordingly.
	///
	/// Lengths will never exceed `limit` nor `ZOPFLI_MAX_MATCH`, but they
	/// might be _less_ than `ZOPFLI_MIN_MATCH`, especially near the end of a
	/// slice.
	fn find(
		&self,
		arr: &[u8],
		pos: usize,
		mut limit: usize,
		sublen: &mut [u16],
		distance: &mut u16,
		length: &mut u16,
		cache: Option<usize>,
	) -> Result<(), ZopfliError> {
		// Check the longest match cache first!
		if let Some(blockstart) = cache {
			if CACHE.with_borrow(|c| c.find(
				pos - blockstart,
				&mut limit,
				sublen,
				distance,
				length,
			))? {
				if pos + usize::from(*length) <= arr.len() { return Ok(()); }
				return Err(ZopfliError::MatchRange);
			}
		}

		// These are both hard-coded or asserted by the caller.
		debug_assert!((ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH).contains(&limit));

		// We'll need at least ZOPFLI_MIN_MATCH bytes for a search; if we don't
		// have it, zero everything out and call it a day.
		if pos + ZOPFLI_MIN_MATCH > arr.len() {
			*length = 0;
			*distance = 0;
			return Ok(());
		}

		// Cap the limit to fit if needed. Note that limit will always be at
		// least one even if capped since pos < size.
		if pos + limit > arr.len() { limit = arr.len() - pos; }

		// Calculate the best distance and length.
		let (bestdist, bestlength) = self.find_loop(arr, pos, limit, sublen)?;

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
		if pos + usize::from(*length) <= arr.len() { Ok(()) }
		else {
			Err(ZopfliError::MatchRange)
		}
	}

	#[allow(
		clippy::cast_possible_truncation,
		clippy::cast_possible_wrap,
		clippy::cast_sign_loss,
		clippy::similar_names,
	)]
	/// # Find Longest Match Loop.
	///
	/// This method is the (nasty-looking) workhorse of the above
	/// `ZopfliHash::find` method. It finds and returns the matching distance
	/// and length, or `(0, 1)` if none.
	fn find_loop(
		&self,
		arr: &[u8],
		pos: usize,
		limit: usize,
		sublen: &mut [u16],
	) -> Result<(u16, u16), ZopfliError> {
		// This is asserted by find() too, but it's a good reminder.
		debug_assert!(limit <= ZOPFLI_MAX_MATCH);

		// Help the compiler understand sublen has a fixed size if provided.
		// (We can't do an Option<Array> because it's too big for Copy.)
		if ! sublen.is_empty() && sublen.len() != ZOPFLI_MAX_MATCH + 1 {
			return Err(ZopfliError::SublenLength);
		}

		let hpos = pos & ZOPFLI_WINDOW_MASK;

		// The default distance and length. We'll be wanting 16-bit values for
		// both eventually, but they're used in a lot of indexing so usize is
		// more ergonomical for now.
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
			if 0 < dist && dist <= pos {
				// Now and Then indexes for comparison, always "dist" apart
				// from one another.
				let mut currentlength = 0;
				let mut now_idx = pos;
				let mut then_idx = pos - dist;

				// This search is pointless if the first condition is true, but
				// the compiler prefers that small waste to any sort of if/else
				// break logic.
				if
					now_idx + bestlength >= arr.len() ||
					arr[now_idx + bestlength] == arr[then_idx + bestlength]
				{
					if 2 < same0 && arr[now_idx] == arr[then_idx] {
						let same2 = usize::from(self.same[then_idx & ZOPFLI_WINDOW_MASK]);
						let same = usize::min(same1, same2);
						now_idx += same;
						then_idx += same;
					}

					while
						now_idx < arr.len() &&
						then_idx < arr.len() &&
						now_idx < pos + limit &&
						arr[now_idx] == arr[then_idx]
					{
						now_idx += 1;
						then_idx += 1;
					}

					// The length is the distance now_idx has traveled.
					currentlength = now_idx - pos;
				}

				// We've found a better length!
				if bestlength < currentlength {
					// Update the sublength slice, if provided. Note that
					// sublengths are (ZOPFLI_MAX_MATCH+1) if provided, and
					// ZOPFLI_MAX_MATCH is the largest possible value of
					// currentlength.
					if currentlength < sublen.len() {
						sublen[bestlength + 1..=currentlength].fill(dist as u16);
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
		if bestlength <= limit { Ok((bestdist as u16, bestlength as u16)) }
		else { Ok((0, 1)) }
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

/// # Minimum Cost Model.
///
/// This returns the minimum _statistical_ cost, which is the sum of the
/// minimum length cost and minimum distance cost.
fn get_minimum_cost(stats: &SymbolStats) -> f64 {
	// Find the minimum length cost.
	let mut length_cost = f64::INFINITY;
	for &(lsym, lbits, _) in LENGTH_SYMBOLS_BITS_VALUES.iter().skip(3) {
		let cost = f64::from(lbits) + stats.ll_symbols[lsym as usize];
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



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_fixed_cost() {
		// Get the largest dbit and lbit values.
		let d_max: u8 = DISTANCE_BITS.iter().copied().max().unwrap();
		let l_max: u8 = LENGTH_SYMBOLS_BITS_VALUES.iter()
			.map(|(_, a, _)| *a)
			.max()
			.unwrap();

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
		for (i, (sym, _, _)) in LENGTH_SYMBOLS_BITS_VALUES.iter().copied().enumerate() {
			assert_eq!(
				279 < (sym as u16),
				114 < i,
				"get_fixed_cost() base logic is wrong: len {i} has symbol {}", sym as u16
			);
		}
	}
}
