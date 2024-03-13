/*!
# Flaca: Zopflipng Longest Match Cache.
*/

use std::{
	cell::RefCell,
	os::raw::{
		c_ushort,
		c_uint,
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
#[allow(unsafe_code)]
#[inline]
/// # Fetch Cached Sublength.
///
/// This is a rewrite of the original `cache.c` method.
pub(crate) extern "C" fn ZopfliCacheToSublen(
	pos: usize,
	length: usize,
	sublen: *mut ::std::os::raw::c_ushort,
) {
	// Short circuit.
	if length < ZOPFLI_MIN_MATCH { return; }

	CACHE.with_borrow(|c| {
		// Convert the raw pointer to a slice to make it easier to work with.
		let sublen: &mut [c_ushort] = unsafe {
			std::slice::from_raw_parts_mut(sublen, SUBLEN_LEN)
		};
		let slice = c.sublen_array(pos);
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
	});
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

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Length and Distance of Cached Sublength.
///
/// Because length and distance are always accessed together, they're stored
/// together to reduce boundary-checking overhead.
pub(crate) extern "C" fn ZopfliLongestMatchCacheLD(
	pos: usize,
	len: *mut c_ushort,
	dist: *mut c_ushort,
) {
	CACHE.with_borrow(|c| {
		let [l1, l2, d1, d2] = c.ld[pos].to_le_bytes();
		unsafe {
			*len = u16::from_le_bytes([l1, l2]);
			*dist = u16::from_le_bytes([d1, d2]);
		}
	});
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Set Length and Distance of Cached Sublength.
pub(crate) extern "C" fn ZopfliLongestMatchCacheSetLD(
	pos: usize,
	len: c_ushort,
	dist: c_ushort,
) {
	let [l1, l2] = len.to_le_bytes();
	let [d1, d2] = dist.to_le_bytes();
	CACHE.with_borrow_mut(|c| { c.ld[pos] = u32::from_le_bytes([l1, l2, d1, d2]); });
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Max Cached Sublength.
///
/// This is a rewrite of the original `cache.c` method.
pub(crate) extern "C" fn ZopfliMaxCachedSublen(pos: usize) -> c_uint {
	CACHE.with_borrow(|c| max_sublen(c.sublen_array(pos)))
}

#[no_mangle]
#[allow(unsafe_code, clippy::cast_possible_truncation)]
#[inline]
/// # Add Sublength to Cache.
///
/// This is a rewrite of the original `cache.c` method.
pub(crate) extern "C" fn ZopfliSublenToCache(
	sublen: *const c_ushort,
	pos: usize,
	length: usize,
) {
	// Short circuit.
	let Some(length) = length.checked_sub(ZOPFLI_MIN_MATCH) else { return; };

	CACHE.with_borrow_mut(|c| {
		// Note: we only need part of the sublength data.
		let sublen: &[c_ushort] = unsafe {
			std::slice::from_raw_parts(sublen.add(ZOPFLI_MIN_MATCH), length + 1)
		};
		// Safety: the boundaries have already been checked.
		let start = unsafe { c.sublen.as_mut_ptr().add(SUBLEN_CACHED_LEN * pos) };
		let mut ptr = start;
		let mut written = 0;

		// Write all mismatched pairs.
		for (i, pair) in sublen.windows(2).enumerate() {
			if pair[0] != pair[1] {
				unsafe {
					std::ptr::write(ptr, i as u8);
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
			std::ptr::write(ptr, length as u8);
			std::ptr::copy_nonoverlapping(
				sublen.get_unchecked(sublen.len() - 1).to_le_bytes().as_ptr(),
				ptr.add(1),
				2,
			);

			written += 1;

			// If we didn't fill the cache, redundantly write the last length
			// into the last chunk to make our future lives easier.
			if written <= ZOPFLI_CACHE_LENGTH {
				std::ptr::write(start.add(SUBLEN_CACHED_LEN - 3), length as u8);
			}
		}
	});
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

	#[allow(unsafe_code)]
	/// # Sublength Array.
	///
	/// Return the cached sublength as a fixed array to ease any subsequent
	/// boundary confusion.
	fn sublen_array(&self, pos: usize) -> &[u8; SUBLEN_CACHED_LEN] {
		let start = SUBLEN_CACHED_LEN * pos;
		unsafe { &* (self.sublen.get_unchecked(start..start + SUBLEN_CACHED_LEN).as_ptr().cast()) }
	}
}



/// # Max Sublength.
///
/// Return the maximum sublength length for a given chunk.
const fn max_sublen(slice: &[u8; SUBLEN_CACHED_LEN]) -> u32 {
	if slice[1] == 0 && slice[2] == 0 { 0 }
	else { slice[SUBLEN_CACHED_LEN - 3] as u32 + 3 }
}
