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
	ZOPFLI_MIN_MATCH,
	ZOPFLI_MAX_MATCH,
};

const ZOPFLI_WINDOW_SIZE: usize = 32_768;
const ZOPFLI_WINDOW_MASK: usize = ZOPFLI_WINDOW_SIZE - 1;
const HASH_SHIFT: i32 = 5;
const HASH_MASK: u16 = 32_767;
const ZOPFLI_MAX_CHAIN_HITS: usize = 8192;



thread_local!(
	/// # Static Hash.
	///
	/// There is only ever one instance of the hash active per thread, so we
	/// might as well persist it to save on the allocations!
	static HASH: RefCell<Box<ZopfliHash>> = RefCell::new(ZopfliHash::new())
);



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
#[inline]
#[allow(unsafe_code, clippy::cast_possible_truncation)]
/// # Is Long Repetition?
///
/// Returns true if the position has a long repetition.
///
/// This is only used by `GetBestLengths` in `squeeze.c`; performing this check
/// here enables us to remove all direct traces of the `ZopfliHash` struct from
/// that half of the codebase.
pub(crate) extern "C" fn ZopfliLongRepetition(pos: usize) -> c_uchar {
	HASH.with_borrow(|h| u8::from(
		ZOPFLI_MAX_MATCH <= pos &&
		h.same[pos & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16 * 2 &&
		h.same[(pos - ZOPFLI_MAX_MATCH) & ZOPFLI_WINDOW_MASK] > ZOPFLI_MAX_MATCH as u16
	))
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

	#[allow(
		unsafe_code,
		clippy::cast_possible_truncation,
	)]
	/// # Initialize Values.
	///
	/// Initialize/reset hash values to their defaults so we can reuse the
	/// structure for a new dataset.
	unsafe fn init(&mut self) {
		// The idx<=>hash arrays default to -1 for "None".
		addr_of_mut!(self.chain1.hash_idx).write_bytes(u8::MAX, 1);
		addr_of_mut!(self.chain1.idx_hash).write_bytes(u8::MAX, 1);

		// Each value in the previous array unfortunately defaults to its
		// position, so have to be written one at a time.
		let prev_idx = addr_of_mut!(self.chain1.prev_idx).cast::<u16>();
		for i in 0..ZOPFLI_WINDOW_SIZE {
			prev_idx.add(i).write(i as u16);
		}

		// The initial hash value is just plain zero.
		addr_of_mut!(self.chain1.val).write(0);

		// The second chain is the same as the first, so we can copy it
		// wholesale.
		addr_of_mut!(self.chain2).copy_from_nonoverlapping(addr_of!(self.chain1), 1);

		// The repetition counts all start at zero.
		addr_of_mut!(self.same).write_bytes(0, 1);
	}

	#[allow(
		clippy::cast_possible_truncation,
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
		self.chain2.val = ((amount - ZOPFLI_MIN_MATCH as u16) & 255) ^ self.chain1.val;
		self.chain2.update_hash(pos);
	}

	/// # Update Hash Value.
	///
	/// This updates the rotating (chain1) value.
	fn update_hash_value(&mut self, c: u8) {
		self.chain1.val = ((self.chain1.val << HASH_SHIFT) ^ u16::from(c)) & HASH_MASK;
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
		clippy::cognitive_complexity,
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

		debug_assert_eq!(chain.hash_idx[usize::from(chain.val)], hpos as i16);

		// Masking is unnecessary but helps the compiler know the values are
		// within 0..ZOPFLI_WINDOW_SIZE (for indexing purposes).
		let mut pp = hpos;
		let mut p = usize::from(chain.prev_idx[hpos]);

		// Even though the ultimate distance will be u16, this variable needs
		// to be at least 32-bits to deal with overflowing math.
		let mut dist =
			if p < pp { pp - p }
			else { ZOPFLI_WINDOW_SIZE + pp - p };

		let mut hits = 0;
		let same0 = usize::from(self.same[hpos]);
		let same1 = usize::min(same0, limit);
		while p < ZOPFLI_WINDOW_SIZE && dist < ZOPFLI_WINDOW_SIZE && hits < ZOPFLI_MAX_CHAIN_HITS {
			let mut currentlength = 0;

			// These are simple sanity assertions;
			debug_assert_eq!(p, usize::from(chain.prev_idx[pp]));
			debug_assert_eq!(chain.idx_hash[p], chain.val as i16);

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
				self.chain2.idx_hash[p] == self.chain2.val as i16
			{
				switched = true;
				chain = &self.chain2;
			}

			// Reset the reference points for the next iteration.
			pp = p;
			p = usize::from(chain.prev_idx[p]);

			// Stop early if we've run out of matching tails.
			if p == pp { break; }

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
/// In the original C, these four values are repeated twice in the main struct
/// with names like `head`/`head2`, `prev`/`prev2`, etc.
///
/// This simply abstracts the collection to its own chain to improve the
/// efficiency.
///
/// Note that most of the integer types are larger than necessary for their
/// payloads as that works out better for performance (for mysterious
/// reasons).
pub(crate) struct ZopfliHashChain {
	/// Hash value to (most recent) index.
	///
	/// -1 is used for "None"; otherwise this is basically half a u16.
	///
	/// Note: the original (head/head2) `hash.c` implementation was
	/// over-allocated for some reason; the hash values are masked like
	/// everything else so won't exceed 0..ZOPFLI_WINDOW_SIZE.
	hash_idx: [i16; ZOPFLI_WINDOW_SIZE],

	/// Index to hash value (if any); this is the reverse of `hash_idx`.
	///
	/// -1 is used for "None"; otherwise this is basically half a u16.
	idx_hash: [i16; ZOPFLI_WINDOW_SIZE],

	/// Index to the previous index with the same hash.
	///
	/// This has the same range as the hash indexes, but without -1 since its
	/// default values match the index (e.g. 0, 1, 2, 3, 4…).
	prev_idx: [u16; ZOPFLI_WINDOW_SIZE],

	/// Current hash value.
	///
	/// Again, only half the u16 range is actually used.
	val: u16,
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
		let hval = self.val & HASH_MASK; // Masked for the compiler; self.val is always in range.

		// Update the hash.
		self.idx_hash[hpos] = hval as i16;

		// Update the tail.
		let hash_idx = self.hash_idx[usize::from(hval)];
		self.prev_idx[hpos] =
			if 0 <= hash_idx && self.idx_hash[hash_idx as usize] == hval as i16 {
				hash_idx as u16
			}
			else { hpos as u16 };

		// Update the head.
		self.hash_idx[usize::from(hval)] = hpos as i16;
	}
}
