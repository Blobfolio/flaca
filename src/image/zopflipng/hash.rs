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
	os::raw::c_uchar,
	ptr::{
		addr_of,
		addr_of_mut,
		NonNull,
	},
};
use super::{
	CACHE,
	SUBLEN_LEN,
	SymbolStats,
	ZOPFLI_MAX_MATCH,
	ZOPFLI_MIN_MATCH,
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
	static HASH: RefCell<Box<ZopfliHash>> = RefCell::new(ZopfliHash::new())
);



#[no_mangle]
#[allow(unsafe_code)]
pub(crate) extern "C" fn GetBestLengths(
	arr: *const u8,
	instart: usize,
	inend: usize,
	stats: *const SymbolStats,
	length_array: *mut u16,
	costs: *mut f32,
) -> f64 {
	// Easy abort.
	if instart >= inend { return 0.0; }

	// Initialize costs and lengths.
	let blocksize = inend - instart;

	let costs = unsafe { std::slice::from_raw_parts_mut(costs, blocksize + 1) };
	costs.fill(f32::INFINITY);
	costs[0] = 0.0;

	let length_array = unsafe { std::slice::from_raw_parts_mut(length_array, blocksize + 1) };
	length_array[0] = 0;

	// Dereference the stats if there are any.
	let stats =
		if stats.is_null() { None }
		else { Some(unsafe { &*stats }) };

	HASH.with_borrow_mut(|h| h.get_best_lengths(
		arr,
		instart,
		inend,
		stats,
		length_array,
		costs,
	))
}

#[no_mangle]
#[inline]
#[allow(unsafe_code)]
/// # Find Longest Match.
///
/// This is a rewrite of the original `lz77.c` method.
pub(crate) extern "C" fn ZopfliFindLongestMatch(
	arr: *const u8,
	pos: usize,
	size: usize,
	limit: usize,
	sublen: *mut u16,
	distance: *mut u16,
	length: *mut u16,
	cache: u8,
	blockstart: usize,
) {
	let sublen: &mut [u16] =
		if sublen.is_null() { &mut [] }
		else {
			unsafe { std::slice::from_raw_parts_mut(sublen, SUBLEN_LEN) }
		};

	HASH.with_borrow(|h| h.find(
		arr,
		pos,
		size,
		limit,
		sublen,
		unsafe { &mut *distance },
		unsafe { &mut *length },
		if cache == 1 { Some(blockstart) } else { None },
	));
}

#[no_mangle]
#[allow(unsafe_code)]
/// # Reset Hash.
///
/// Reset the thread-local Zopfli Hash instance to its default values,
/// "warm up" the hash, and prepopulate entries from windowstart to instart.
pub(crate) extern "C" fn ZopfliResetHash(
	arr: *const c_uchar,
	length: usize,
	windowstart: usize,
	instart: usize,
) {
	HASH.with_borrow_mut(|h| {
		unsafe {
			// Set all values to their defaults.
			h.init();

			// Cycle the hash once or twice.
			h.update_hash_value(*arr.add(windowstart));
			if windowstart + 1 < length {
				h.update_hash_value(*arr.add(windowstart + 1));
			}
		}

		// Process the values between windowstart and instart.
		for i in windowstart..instart {
			h.update_hash(
				unsafe { std::slice::from_raw_parts(arr.add(i), length - i) },
				i,
			);
		}
	});
}

#[no_mangle]
#[allow(unsafe_code)]
/// # Update Hash.
///
/// Add a slice to the hash.
pub(crate) extern "C" fn ZopfliUpdateHash(
	arr: *const c_uchar,
	pos: usize,
	length: usize,
) {
	if pos < length {
		let arr = unsafe { std::slice::from_raw_parts(arr.add(pos), length - pos) };
		HASH.with_borrow_mut(|h| h.update_hash(arr, pos));
	}
}



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
	fn get_best_lengths(
		&mut self,
		arr: *const u8,
		instart: usize,
		inend: usize,
		stats: Option<&SymbolStats>,
		length_array: &mut [u16],
		costs: &mut [f32],
	) -> f64 {
		let windowstart = instart.saturating_sub(ZOPFLI_WINDOW_SIZE);

		// Reset and warm the hash.
		unsafe {
			self.init();
			self.update_hash_value(*arr.add(windowstart));
			if windowstart + 1 < inend {
				self.update_hash_value(*arr.add(windowstart + 1));
			}
		}

		let mut length = 0_u16;
		let mut distance = 0_u16;
		let mut sublen = [0_u16; SUBLEN_LEN];

		// Find the minimum and maximum cost.
		let min_cost = stats.map_or(12.0, get_minimum_cost);

		// Convert the array to a slice for safer reslicing.
		let arr = unsafe { std::slice::from_raw_parts(arr, inend) };
		let mut i = windowstart;
		while i < arr.len() {
			// Hash the remainder.
			self.update_hash(&arr[i..], i);
			if i < instart {
				i += 1;
				continue;
			}

			// Relative position for both the costs and lengths arrays; these
			// contain (iend - istart + 1) entries, so anytime i is in range
			// for arr, j is in range for costs and length_array.
			let mut j = i - instart;

			// We're in a long repetition of the same character and have more
			// than ZOPFLI_MAX_MATCH ahead of and behind us.
			if
				self.same[i & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16 * 2 &&
				i > instart + ZOPFLI_MAX_MATCH + 1 &&
				arr.len() > i + ZOPFLI_MAX_MATCH * 2 + 1 &&
				self.same[(i - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16
			{
				// Set the lengths of each repetition to ZOPFLI_MAX_MATCH, and
				// the cost to the (precalculated) cost of that length.
				let symbol_cost = stats.map_or(
					13.0,
					|s| (s.ll_symbols[285] + s.d_symbols[0]),
				);
				for _ in 0..ZOPFLI_MAX_MATCH {
					// Safety: we verified at least ZOPFLI_MAX_MATCH entries
					// remain in arr, so that many plus one remain in the cost
					// and length arrays too.
					unsafe {
						*costs.get_unchecked_mut(j + ZOPFLI_MAX_MATCH) = (
							f64::from(*costs.get_unchecked(j)) + symbol_cost
						) as f32;
						*length_array.get_unchecked_mut(j + ZOPFLI_MAX_MATCH) = ZOPFLI_MAX_MATCH as u16;
					}
					i += 1;
					j += 1;
					self.update_hash(&arr[i..], i);
				}
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

			// Literal.
			if i < arr.len() {
				let new_cost = stats.map_or(
					if arr[i] <= 143 { 8.0 } else { 9.0 },
					|s| s.ll_symbols[usize::from(arr[i])],
				) + f64::from(unsafe { *costs.get_unchecked(j) });
				debug_assert!(0.0 <= new_cost);

				// Update it if lower.
				if new_cost < f64::from(unsafe { *costs.get_unchecked(j + 1) }) {
					costs[j + 1] = new_cost as f32;
					length_array[j + 1] = 1;
				}
			}

			// Lengths and Sublengths.
			let limit = usize::from(length).min(arr.len() - i);
			if (ZOPFLI_MIN_MATCH..=ZOPFLI_MAX_MATCH).contains(&limit) {
				let min_cost_add = min_cost + f64::from(unsafe { *costs.get_unchecked(j) });
				let mut k = ZOPFLI_MIN_MATCH;
				for &v in &sublen[ZOPFLI_MIN_MATCH..=limit] {
					// The expensive cost calculations are only worth
					// performing if the stored cost is larger than the
					// minimum cost we found earlier.
					if min_cost_add < f64::from(unsafe { *costs.get_unchecked(j + k) }) {
						let new_cost = stats.map_or_else(
							|| get_fixed_cost(k as u16, v),
							|s| get_stat_cost(k as u16, v, s),
						) + f64::from(costs[j]);
						debug_assert!(0.0 <= new_cost);

						// Update it if lower.
						if new_cost < f64::from(costs[j + k]) {
							costs[j + k] = new_cost as f32;
							length_array[j + k] = k as u16;
						}
					}
					k += 1;
				}
			}

			// Back around again!
			i += 1;
		}

		// Return the final cost!
		debug_assert!(0.0 <= costs[costs.len() - 1]);
		f64::from(costs[costs.len() - 1])
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
