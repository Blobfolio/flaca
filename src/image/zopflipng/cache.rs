/*!
# Flaca: Zopflipng Longest Match Cache.
*/

use std::{
	cell::RefCell,
	os::raw::c_ushort,
};
use super::{
	ZOPFLI_MIN_MATCH,
	ZOPFLI_MAX_MATCH,
};



thread_local!(
	/// # Static Cache.
	///
	/// There is only ever one instance of the LZ77 cache active per thread,
	/// so we might as well persist it to save on the allocations!
	pub(crate) static CACHE: RefCell<MatchCache> = const { RefCell::new(MatchCache::new()) };

	/// # Static Squeeze Scratch.
	///
	/// Similar to the above, the costs, lengths, and paths crunched during the
	/// squeeze passes never overlap so can be shared to reduce allocation
	/// overhead.
	pub(crate) static SQUEEZE: RefCell<SqueezeCache> = const { RefCell::new(SqueezeCache::new()) };
);

/// # Default Length (1) and Distance (0).
///
/// Length and distance are always fetched together, so are stored together to
/// halve the bounds-checking overhead.
const DEFAULT_LD: u32 = u32::from_le_bytes([1, 0, 0, 0]);

const ZOPFLI_CACHE_LENGTH: usize = 8;

/// # Length of Cached Sublength Slice.
const SUBLEN_CACHED_LEN: usize = ZOPFLI_CACHE_LENGTH * 3;



/// # Longest Match Cache.
///
/// This structure holds cached length/distance details for individual
/// sublengths. Its memory usage is no joke, but the performance savings more
/// than make up for it.
pub(crate) struct MatchCache {
	ld: Vec<u32>,
	sublen: Vec<u8>,
}

impl MatchCache {
	/// # New.
	const fn new() -> Self {
		Self {
			ld: Vec::new(),
			sublen: Vec::new(),
		}
	}

	/// # Initialize.
	///
	/// This resizes the cache buffers and resets their values to their default
	/// states — one for length, zero for everything else.
	///
	/// Because this is a shared buffer, allocations persist for the duration
	/// of the program run so they can be reused.
	pub(crate) fn init(&mut self, blocksize: usize) {
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
	pub(crate) fn find(
		&self,
		pos: usize,
		limit: &mut usize,
		sublen: &mut [u16],
		distance: &mut u16,
		length: &mut u16,
	) -> bool {
		// If we have no distance, we have no cache.
		let (cache_len, cache_dist) = self.ld(pos);
		if cache_len != 0 && cache_dist == 0 { return false; }

		// Find the max sublength once, if ever.
		let maxlength =
			if sublen.is_empty() { 0 }
			else { max_sublen(self.sublen_array(pos)) };

		// Proceed if our cached length or max sublength are under the limit.
		if
			*limit == ZOPFLI_MAX_MATCH ||
			usize::from(cache_len) <= *limit ||
			(! sublen.is_empty() && maxlength >= *limit)
		{
			// Update length and distance if the sublength pointer is null or
			// the cached sublength is bigger than the cached length.
			if sublen.is_empty() || usize::from(cache_len) <= maxlength {
				// Cap the length.
				*length = cache_len;
				if usize::from(*length) > *limit {
					*length = *limit as u16;
				}

				// Use the cached distance directly.
				if sublen.is_empty() {
					*distance = cache_dist;
				}
				else {
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

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Set Sublength.
	pub(crate) fn set_sublen(
		&mut self,
		pos: usize,
		sublen: &[u16],
		distance: c_ushort,
		length: c_ushort,
	) {
		let (cache_len, cache_dist) = self.ld(pos);
		if cache_len == 0 || cache_dist != 0 { return; }
		debug_assert_eq!(
			(cache_len, cache_dist),
			(1, 0),
			"Length and/or distance are already cached!"
		);

		// The sublength isn't cacheable, but that fact is itself worth
		// caching!
		if usize::from(length) < ZOPFLI_MIN_MATCH {
			self.set_ld(pos, 0, 0);
			return;
		}

		// Save the length/distance bit.
		debug_assert_ne!(
			distance,
			0,
			"Distance cannot be zero when length > ZOPFLI_MIN_MATCH!"
		);
		self.set_ld(pos, length, distance);

		// Convert (the relevant) part of the sublength to a slice to make
		// it easier to work with.
		let start = unsafe { self.sublen.as_mut_ptr().add(SUBLEN_CACHED_LEN * pos) };
		let mut ptr = start;
		let mut written = 0;

		// Write all mismatched pairs.
		for (i, pair) in sublen.windows(2).skip(ZOPFLI_MIN_MATCH).take(usize::from(length) - 3).enumerate() {
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
			ptr.write((length - 3) as u8);
			std::ptr::copy_nonoverlapping(
				sublen.get_unchecked(length as usize).to_le_bytes().as_ptr(),
				ptr.add(1),
				2,
			);

			written += 1;

			// If we didn't fill the cache, redundantly write the last length
			// into the last chunk to make our future lives easier.
			if written < ZOPFLI_CACHE_LENGTH {
				start.add(SUBLEN_CACHED_LEN - 3).write((length - 3) as u8);
			}
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

	#[allow(unsafe_code)]
	/// # Write Sublength.
	fn write_sublen(&self, pos: usize, length: usize, sublen: &mut [u16]) {
		// Short circuit.
		if length < ZOPFLI_MIN_MATCH { return; }

		let slice = self.sublen_array(pos);
		let maxlength = max_sublen(slice);
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



/// # Squeeze Scratchpad.
///
/// This structure is used to keep track of the data gathered during the
/// forward/backward "squeeze" passes.
///
/// It is initialized with the `MatchCache` because they're used together, but
/// kept separate to mitigate lock contention.
pub(crate) struct SqueezeCache {
	pub(crate) costs: Vec<(f32, u16)>,
	pub(crate) paths: Vec<u16>,
}

impl SqueezeCache {
	/// # New.
	const fn new() -> Self {
		Self {
			costs: Vec::new(),
			paths: Vec::new(),
		}
	}

	#[inline]
	/// # Initialize/Reset.
	///
	/// This (potentially) resizes the cost and length vectors for the given
	/// blocksize — which is `(inend - instart + 1)` by the way.
	///
	/// Unlike the `MatchCache`, this doesn't worry about setting the
	/// appropriate values; `SqueezeCache::reset_costs` handles that.
	///
	/// The paths are unchanged by this method; subsequent calls to
	/// `SqueezeCache::trace_paths` gets them sorted.
	pub(crate) fn init(&mut self, blocksize: usize) {
		// Resize if needed.
		if blocksize != self.costs.len() {
			self.costs.resize(blocksize, (f32::INFINITY, 0));
		}
	}

	#[inline]
	/// # Reset Costs.
	///
	/// This nudges all costs to infinity except the first, which is set to
	/// zero instead.
	pub(crate) fn reset_costs(&mut self) {
		if ! self.costs.is_empty() {
			for c in &mut self.costs { c.0 = f32::INFINITY; }
			self.costs[0].0 = 0.0;
		}
	}

	#[allow(unsafe_code, clippy::cast_possible_truncation)]
	/// # Trace Paths.
	///
	/// Calculate the optimal path of lz77 lengths to use, from the
	/// lengths gathered during the `ZopfliHash::get_best_lengths` pass.
	pub(crate) fn trace_paths(&mut self) {
		// Kill any previous paths, if any.
		self.paths.truncate(0);

		let len = self.costs.len();
		if len < 2 { return; }

		let mut idx = len - 1;
		while 0 < idx {
			let v = self.costs[idx].1;
			assert!((1..=ZOPFLI_MAX_MATCH as u16).contains(&v));

			// Only lengths of at least ZOPFLI_MIN_MATCH count as lengths
			// after tracing.
			self.paths.push(
				if v < ZOPFLI_MIN_MATCH as u16 { 1 } else { v }
			);

			// Move onto the next length or finish.
			idx = idx.saturating_sub(usize::from(v));
		}
	}
}



/// # Max Sublength.
///
/// Return the maximum sublength length for a given chunk.
const fn max_sublen(slice: &[u8; SUBLEN_CACHED_LEN]) -> usize {
	if slice[1] == 0 && slice[2] == 0 { 0 }
	else { slice[SUBLEN_CACHED_LEN - 3] as usize + 3 }
}
