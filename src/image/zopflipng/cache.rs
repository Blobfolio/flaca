/*!
# Flaca: Zopflipng Longest Match Cache.
*/

use std::{
	cell::RefCell,
	os::raw::{
		c_int,
		c_ushort,
	},
};



thread_local!(
	/// # Static Cache.
	///
	/// There is only ever one instance of the LZ77 cache active per thread,
	/// so we might as well persist it to save on the allocations!
	static CACHE: RefCell<MatchCache> = RefCell::new(MatchCache::default())
);

/// # Default Length (1) and Distance (0).
///
/// Length and distance are always fetched together, so are stored together to
/// halve the bounds-checking overhead.
const DEFAULT_LD: u32 = u32::from_le_bytes([1, 0, 0, 0]);

const ZOPFLI_CACHE_LENGTH: usize = 8;
const ZOPFLI_MAX_MATCH: usize = 258;
const ZOPFLI_MIN_MATCH: usize = 3;

/// # Length of Sublength Array.
///
/// This is hardcoded in `squeeze.c`.
const SUBLEN_LEN: usize = ZOPFLI_MAX_MATCH + 1;

/// # Length of Cached Sublength Slice.
const SUBLEN_CACHED_LEN: usize = ZOPFLI_CACHE_LENGTH * 3;



#[no_mangle]
#[inline]
#[allow(unsafe_code, clippy::cast_possible_truncation)]
/// # Save Length, Distance, and/or Sublength to Cache.
///
/// This is a rewrite of the original `lz77.c` method.
pub(crate) extern "C" fn StoreInLongestMatchCache(
	pos: usize,
	sublen: *const c_ushort,
	distance: c_ushort,
	length: c_ushort,
) {
	CACHE.with_borrow_mut(|c| {
		let (cache_len, cache_dist) = c.ld(pos);
		if cache_len == 0 || cache_dist != 0 { return; }
		debug_assert_eq!(
			(cache_len, cache_dist),
			(1, 0),
			"Length and/or distance are already cached!"
		);

		// The sublength isn't cacheable, but that fact is itself worth
		// caching!
		if usize::from(length) < ZOPFLI_MIN_MATCH {
			c.set_ld(pos, 0, 0);
			return;
		}

		// Save the length/distance bit.
		debug_assert_ne!(
			distance,
			0,
			"Distance cannot be zero when length > ZOPFLI_MIN_MATCH!"
		);
		c.set_ld(pos, length, distance);

		// Convert (the relevant) part of the sublength to a slice to make
		// it easier to work with.
		let Some(length) = usize::from(length).checked_sub(ZOPFLI_MIN_MATCH) else { unreachable!() };
		let sublen: &[c_ushort] = unsafe {
			std::slice::from_raw_parts(sublen.add(ZOPFLI_MIN_MATCH), length + 1)
		};
		let start = unsafe { c.sublen.as_mut_ptr().add(SUBLEN_CACHED_LEN * pos) };
		let mut ptr = start;
		let mut written = 0;

		// Write all mismatched pairs.
		for (i, pair) in sublen.windows(2).enumerate() {
			if pair[0] != pair[1] {
				unsafe {
					ptr.write(i as u8);
					std::ptr::copy_nonoverlapping(
						pair[0].to_le_bytes().as_ptr(),
						ptr.add(1),
						2,
					);

					written += 1;
					if written == ZOPFLI_CACHE_LENGTH { return; }

					ptr = ptr.add(3);
				}
			}
		}

		// Write the final length/distance.
		unsafe {
			ptr.write(length as u8);
			std::ptr::copy_nonoverlapping(
				sublen.get_unchecked(sublen.len() - 1).to_le_bytes().as_ptr(),
				ptr.add(1),
				2,
			);

			written += 1;

			// If we didn't fill the cache, redundantly write the last length
			// into the last chunk to make our future lives easier.
			if written < ZOPFLI_CACHE_LENGTH {
				start.add(SUBLEN_CACHED_LEN - 3).write(length as u8);
			}
		}
	});
}

#[no_mangle]
#[inline]
#[allow(unsafe_code, clippy::cast_possible_truncation)]
/// # Maybe Find From Cache.
///
/// This is a rewrite of the original `lz77.c` method.
pub(crate) extern "C" fn TryGetFromLongestMatchCache(
	pos: usize,
	limit: *mut usize,
	sublen: *mut c_ushort,
	distance: *mut c_ushort,
	length: *mut c_ushort,
) -> c_int {
	CACHE.with_borrow(|c| {
		let res = c.find(
			pos,
			unsafe { &mut *limit },
			sublen,
			unsafe { &mut *distance },
			unsafe { &mut *length }
		);
		i32::from(res)
	})
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Initialize Longest Match Cache.
///
/// This is a rewrite of the original `cache.c` method.
pub(crate) extern "C" fn ZopfliInitCache(blocksize: usize) {
	CACHE.with_borrow_mut(|c| { c.init(blocksize); });
}



#[derive(Default)]
/// # Longest Match Cache.
///
/// This structure holds cached length/distance details for individual
/// sublengths. Its memory usage is no joke, but the performance savings more
/// than make up for it.
struct MatchCache {
	ld: Vec<u32>,
	sublen: Vec<u8>,
}

impl MatchCache {
	/// # Initialize.
	///
	/// This resizes the cache buffers and resets their values to their default
	/// states — one for length, zero for everything else.
	///
	/// Because this is a shared buffer, allocations persist for the duration
	/// of the program run so they can be reused.
	fn init(&mut self, blocksize: usize) {
		let mut old_blocksize = self.ld.len();

		// Shrink to fit.
		if old_blocksize > blocksize {
			self.ld.truncate(blocksize);
			self.sublen.truncate(blocksize * SUBLEN_CACHED_LEN);
			old_blocksize = blocksize;
		}

		// Fill existing slots.
		if old_blocksize != 0 {
			self.ld.fill(DEFAULT_LD);
			self.sublen.fill(0);
		}

		// Expand as needed.
		if blocksize > old_blocksize {
			self.ld.resize(blocksize, DEFAULT_LD);
			self.sublen.resize(blocksize * SUBLEN_CACHED_LEN, 0);
		}
	}

	#[inline]
	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Find Match.
	///
	/// Find the sublength, distance, and length from cache, if possible.
	///
	/// Values are written directly to the passed arguments. A bool is returned
	/// to indicate whether or not the find was successful.
	///
	/// Note: All the mismatched integer types are Zopfli's fault. Haha.
	fn find(
		&self,
		pos: usize,
		limit: &mut usize,
		sublen: *mut c_ushort,
		distance: &mut u16,
		length: &mut u16,
	) -> bool {
		// If we have no distance, we have no cache.
		let (cache_len, cache_dist) = self.ld(pos);
		if cache_len != 0 && cache_dist == 0 { return false; }

		// Proceed if our cached length or max sublength are under the limit.
		if
			*limit == ZOPFLI_MAX_MATCH ||
			usize::from(cache_len) <= *limit ||
			(
				! sublen.is_null() &&
				max_sublen(self.sublen_array(pos)) as usize >= *limit
			)
		{
			// Update length and distance if the sublength pointer is null or
			// the cached sublength is bigger than the cached length.
			if sublen.is_null() || u32::from(cache_len) <= max_sublen(self.sublen_array(pos)) {
				// Cap the length.
				*length = cache_len;
				if usize::from(*length) > *limit {
					*length = *limit as u16;
				}

				// Use the cached distance directly.
				if sublen.is_null() {
					*distance = cache_dist;
				}
				else {
					// Convert the raw pointer to a slice.
					let sublen: &mut [c_ushort] = unsafe {
						std::slice::from_raw_parts_mut(sublen, SUBLEN_LEN)
					};

					// Pull the sublength from cache and pull the distance from
					// that.
					self.write_sublen(pos, usize::from(*length), sublen);
					*distance = sublen[usize::from(*length)];

					// Sanity check: make sure the sublength distance at length
					// matches the redundantly-cached distance.
					if *limit == ZOPFLI_MAX_MATCH && usize::from(*length) >= ZOPFLI_MIN_MATCH {
						assert_eq!(*distance, cache_dist);
					}
				}

				// We did stuff!
				return true;
			}

			// Replace the limit with our sad cached length.
			*limit = usize::from(cache_len);
		}

		// Nothing happened.
		false
	}

	/// # Get Length and Distance.
	fn ld(&self, pos: usize) -> (u16, u16) {
		let [l1, l2, d1, d2] = self.ld[pos].to_le_bytes();
		(u16::from_le_bytes([l1, l2]), u16::from_le_bytes([d1, d2]))
	}

	/// # Set Length and Distance.
	fn set_ld(&mut self, pos: usize, len: c_ushort, dist: c_ushort) {
		let [l1, l2] = len.to_le_bytes();
		let [d1, d2] = dist.to_le_bytes();
		self.ld[pos] = u32::from_le_bytes([l1, l2, d1, d2]);
	}

	#[allow(unsafe_code)]
	/// # Sublength Array.
	///
	/// Return the cached sublength as a fixed array to ease any subsequent
	/// boundary confusion.
	fn sublen_array(&self, pos: usize) -> &[u8; SUBLEN_CACHED_LEN] {
		let start = SUBLEN_CACHED_LEN * pos;
		unsafe { &* (self.sublen.get_unchecked(start..start + SUBLEN_CACHED_LEN).as_ptr().cast()) }
	}

	#[allow(unsafe_code)]
	/// # Write Sublength.
	fn write_sublen(&self, pos: usize, length: usize, sublen: &mut [c_ushort]) {
		// Short circuit.
		if length < ZOPFLI_MIN_MATCH { return; }

		let slice = self.sublen_array(pos);
		let maxlength = max_sublen(slice) as usize;
		let mut prevlength = 0;

		for chunk in slice.chunks_exact(3) {
			let length = usize::from(chunk[0]) + ZOPFLI_MIN_MATCH;
			if prevlength <= length {
				let dist = u16::from_le_bytes([chunk[1], chunk[2]]);
				// Safety: these positions existed before.
				unsafe { sublen.get_unchecked_mut(prevlength..=length).fill(dist); }
			}
			if length == maxlength { return; }
			prevlength = length + 1;
		}
	}
}



/// # Max Sublength.
///
/// Return the maximum sublength length for a given chunk.
const fn max_sublen(slice: &[u8; SUBLEN_CACHED_LEN]) -> u32 {
	if slice[1] == 0 && slice[2] == 0 { 0 }
	else { slice[SUBLEN_CACHED_LEN - 3] as u32 + 3 }
}
