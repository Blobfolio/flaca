/*!
# Flapfli: Caches.

This module contains the Longest Match cache along with several smaller caching
structures that aren't big enough to warrant their own dedicated modules.
*/

use std::cell::Cell;
use super::{
	LitLen,
	SUBLEN_LEN,
	zopfli_error,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZOPFLI_MIN_MATCH,
	ZopfliChunk,
	ZopfliError,
	ZopfliRange,
};



/// # Default Length (1) and Distance (0).
///
/// Length and distance are always fetched/stored together, so are grouped into
/// a single value to reduce indexing/bounds overhead.
///
/// A tuple would be friendlier, but doesn't scale particularly well, so
/// whatever. The `join_ld` and `split_ld` helper methods fill the ergonomic
/// gaps.
const DEFAULT_LD: u32 = u32::from_le_bytes([1, 0, 0, 0]);

/// # Sublength Cache Entries.
///
/// This is the total number of "entries" a given sublength cache record
/// contains.
const ZOPFLI_CACHE_LENGTH: usize = 8;

/// # Sublength Cache Total Length.
///
/// Each entry uses three bytes, so the total length of a sublength cache
/// collection is thus…
const SUBLEN_CACHED_LEN: usize = ZOPFLI_CACHE_LENGTH * 3;

/// # Length of Split Cache.
///
/// The split cache is mercifully boolean, so we can pack it into a bit array,
/// reducing its size to one eighth what it otherwise would be.
const SPLIT_CACHE_LEN: usize = ZOPFLI_MASTER_BLOCK_SIZE.div_ceil(8);



#[repr(C)]
/// # Longest Match Cache.
///
/// This structure holds cached length/distance details for individual
/// "sublengths" — chunks of chunks of data processed by `ZopfliHash` —
/// mitigating the overhead of doing the same shit over and over and over
/// again.
///
/// As with most of this library's caches, the memory usage is no joke, but
/// trying to get by without without it is downright _miserable_.
///
/// On the bright side, we only need one instance per thread for the duration
/// of the program run, and thanks to some clever boxing, it winds up on the
/// heap instead of the stack.
pub(crate) struct MatchCache {
	/// # Length and Distance.
	///
	/// Each pair consists of two sixteen-bit values, joined into a single
	/// little endian `u32`.
	ld: [u32; ZOPFLI_MASTER_BLOCK_SIZE],

	/// # Sublength Cache.
	sublen: [u8; SUBLEN_CACHED_LEN * ZOPFLI_MASTER_BLOCK_SIZE],
}

impl MatchCache {
	/// # Initialize.
	///
	/// Reset (enough of) the cache to its initial/default state for any
	/// subsequent processing of `chunk` we might need to do. (Most chunks will
	/// be smaller than `ZOPFLI_MASTER_BLOCK_SIZE` so we won't normally need to
	/// reset _everything_.)
	///
	/// The length half of `ld` defaults to one; everything else defaults to
	/// zero.
	pub(crate) fn init(&mut self, chunk: &ZopfliChunk<'_>) {
		let blocksize = usize::min(chunk.block_size().get(), ZOPFLI_MASTER_BLOCK_SIZE);

		// Lengths default to one, everything else to zero.
		self.ld[..blocksize].fill(DEFAULT_LD);
		self.sublen[..blocksize * SUBLEN_CACHED_LEN].fill(0);
	}

	/// # Find Match.
	///
	/// Find the sublength, distance, and length from cache, if present, and
	/// (possibly) add it to the cache if not.
	///
	/// The results are written back to the mutable arguments passed to the
	/// method. A bool is returned to indicate whether or not the search was
	/// successful.
	pub(crate) fn find(
		&self,
		pos: usize,
		limit: &mut LitLen,
		sublen: &mut Option<&mut [u16; SUBLEN_LEN]>,
		distance: &mut u16,
		length: &mut LitLen,
	) -> Result<bool, ZopfliError> {
		// One sanity check to rule them all.
		if pos >= ZOPFLI_MASTER_BLOCK_SIZE { return Err(zopfli_error!()); }

		// If we have no distance, we have no cache.
		let (cache_len, cache_dist) = ld_split(self.ld[pos]);
		if ! cache_len.is_zero() && cache_dist == 0 { return Ok(false); }
		let cache_sublen: &[u8; SUBLEN_CACHED_LEN] = self.sublen[pos * SUBLEN_CACHED_LEN..(pos + 1) * SUBLEN_CACHED_LEN]
			.first_chunk::<SUBLEN_CACHED_LEN>()
			.ok_or(zopfli_error!())?;

		// Find the max sublength once, if ever.
		let maxlength =
			if sublen.is_none() { LitLen::L000 }
			else { max_sublen(cache_sublen) };

		// Proceed if our cached length or max sublength are under the limit.
		if
			limit.is_max() ||
			(cache_len as u16) <= (*limit as u16) ||
			(sublen.is_some() && (maxlength as u16) >= (*limit as u16))
		{
			// Update length and distance if the sublength pointer is null or
			// the cached sublength is bigger than the cached length.
			if sublen.is_none() || (cache_len as u16) <= (maxlength as u16) {
				// Cap the length.
				*length = cache_len;
				if (*length as u16) > (*limit as u16) { *length = *limit; }

				// Set the distance from the sublength cache.
				if let Some(s) = sublen {
					// Pull the sublength from cache and pull the distance from
					// that.
					if 3 <= (*length as u16) { write_sublen(cache_sublen, s); }
					*distance = s[*length as usize];

					// Sanity check: make sure the sublength distance at length
					// matches the redundantly-cached distance.
					if
						*distance != cache_dist &&
						limit.is_max() &&
						length.is_matchable()
					{
						return Err(zopfli_error!());
					}
				}
				// Use the cached distance directly.
				else { *distance = cache_dist; }

				// We did stuff!
				return Ok(true);
			}

			// Replace the limit with our sad cached length.
			*limit = cache_len;
		}

		// Nothing happened.
		Ok(false)
	}

	#[cold]
	/// # Set Sublength.
	///
	/// Save the provided sublength data to the cache.
	pub(crate) fn set_sublen(
		&mut self,
		pos: usize,
		sublen: &[u16; SUBLEN_LEN],
		distance: u16,
		length: LitLen,
	) -> Result<(), ZopfliError> {
		if pos >= ZOPFLI_MASTER_BLOCK_SIZE { return Err(zopfli_error!()); }

		// Cache is only worth setting if the current length/distance is the
		// default.
		if DEFAULT_LD != self.ld[pos] {
			let (cache_len, cache_dist) = ld_split(self.ld[pos]);

			// If we previously determined it was not cache-worthy, simply call
			// it a day!
			if cache_dist != 0 || cache_len.is_zero() { return Ok(()); }

			// Otherwise something weird has happened!
			return Err(zopfli_error!());
		}

		// The sublength isn't cacheable, but that fact is itself worth
		// caching!
		if ! length.is_matchable() {
			self.ld[pos] = 0;
			return Ok(());
		}

		// Save the length and distance, unless the distance is zero.
		if distance == 0 { return Err(zopfli_error!()); }
		self.ld[pos] = ld_join(length, distance);

		// Reslice it to the (inclusive) length, ignoring the first 3 entries
		// since they're below the minimum give-a-shittable limit. Note that
		// without them, each index can be represented (and stored) as a u8.
		let slice = &sublen[ZOPFLI_MIN_MATCH..=(length as usize)];

		// The cache gets written three bytes at a time; this iterator will
		// help us eliminate the bounds checks we'd otherwise run into.
		let mut dst = self.sublen.chunks_exact_mut(3).skip(pos * ZOPFLI_CACHE_LENGTH).take(ZOPFLI_CACHE_LENGTH);

		// Start by writing all mismatched pairs, up to the limit.
		for (i, pair) in (0_u8..=u8::MAX).zip(slice.windows(2)) {
			if pair[0] != pair[1] {
				let Some([d0, d1, d2]) = dst.next() else { return Ok(()); };
				*d0 = i;
				[*d1, *d2] = pair[0].to_le_bytes();
			}
		}

		// The final value is implicitly "mismatched"; if we haven't hit the
		// limit we should write it too.
		if let Some([d0, d1, d2]) = dst.next() {
			*d0 = length.to_packed_u8();
			[*d1, *d2] = slice[slice.len() - 1].to_le_bytes();

			// If we're still below the limit, copy (only) the length to the
			// last slot to simplify any subsequent max_length lookups.
			if let Some([d0, _, _]) = dst.last() { *d0 = length.to_packed_u8(); }
		}

		Ok(())
	}
}



/// # Split Cache.
///
/// This structure holds a sort of bit-array used for keeping track of which
/// split points (indices) have already been tested to avoid the overhead of
/// testing them again.
///
/// As with `MatchCache`, we only need one instance of this struct per thread
/// for the duration of the program run.
pub(crate) struct SplitCache {
	/// # Set.
	set: [u8; SPLIT_CACHE_LEN],
}

impl SplitCache {
	/// # Initialize.
	///
	/// Reset the first `rng.len()` bits — these ranges always start at zero —
	/// to false so we can track a new set of indices.
	pub(crate) fn init(&mut self, rng: ZopfliRange) {
		let blocksize = usize::min(rng.len().get(), ZOPFLI_MASTER_BLOCK_SIZE);

		// Fill uses bytes rather than bits, so we need to round up to ensure
		// complete coverage for our range.
		let bitsize = blocksize.div_ceil(8);
		self.set[..bitsize].fill(0);
	}

	#[inline]
	/// # Not Checked?
	///
	/// Returns true if the value is currently _unchecked_. (The caller takes
	/// action on the negative rather than the positive.)
	pub(crate) const fn is_unset(&self, pos: usize) -> bool {
		let idx = pos.wrapping_div(8); // The byte.
		let mask: u8 = 1 << (pos % 8); // The bit.
		SPLIT_CACHE_LEN <= idx || 0 == self.set[idx] & mask
	}

	#[inline]
	/// # Mark as Checked.
	pub(crate) fn set(&mut self, pos: usize) {
		let idx = pos.wrapping_div(8); // The byte.
		let mask: u8 = 1 << (pos % 8); // The bit.
		if idx < SPLIT_CACHE_LEN { self.set[idx] |= mask; }
	}
}



/// # Squeeze Cache.
///
/// This struct stores LZ77 length costs and paths.
///
/// The actual number of costs and paths will vary from image-to-image, block-
/// to-block, but can actually go as high as a million and one!
///
/// Lest that sound like a terrible waste, this struct only exists as part of
/// a thread-local static so will be reused as many times as needed. That
/// static is also boxed to ensure the data winds up on the heap instead of the
/// stack.
pub(crate) struct SqueezeCache {
	/// # Costs and Symbols.
	costs: [(f32, LitLen); ZOPFLI_MASTER_BLOCK_SIZE + 1],

	/// # Paths.
	paths: [LitLen; ZOPFLI_MASTER_BLOCK_SIZE],

	/// # Block Size (+1).
	pub(super) costs_len: Cell<usize>,
}

impl SqueezeCache {
	/// # Resize Costs.
	///
	/// This method merely sets the internal cost-length variable to match
	/// `chunk`'s block size (plus one). (It does _not_ reset the actual
	/// cost data or anything like that.)
	pub(crate) fn resize_costs(&self, chunk: &ZopfliChunk<'_>) {
		self.costs_len.set(chunk.block_size().get() + 1);
	}

	/// # Reset Costs.
	///
	/// Reset and return a mutable slice of costs, sized according to the last
	/// `resize_costs` call.
	///
	/// Note that only the costs themselves are reset; the lengths and paths
	/// are dealt with _in situ_ during crunching (without first being read).
	pub(crate) fn reset_costs(&mut self) -> &mut [(f32, LitLen)] {
		// Clamping shouldn't be necessary as ZopfliChunk verifies the block
		// size is under the limit and non-empty, and since costs is always
		// blocks+1, there'll be at least two.
		let len = self.costs_len.get().clamp(2, ZOPFLI_MASTER_BLOCK_SIZE + 1);

		let costs = &mut self.costs[..len];
		costs[0].0 = 0.0;
		for c in &mut costs[1..] { c.0 = f32::INFINITY; }
		costs
	}

	/// # Trace Paths.
	///
	/// Calculate the optimal path of LZ77 lengths to use given the costs,
	/// returned as a slice.
	///
	/// Note that these are written in reverse order for the benefit of the
	/// `ZopfliHash::follow_paths` call that will wind up using them.
	pub(crate) fn trace_paths(&mut self) -> Result<&[LitLen], ZopfliError> {
		let costs = self.costs.get(..self.costs_len.get()).unwrap_or(&[]);
		if costs.len() < 2 { Ok(&[]) }
		else {
			let mut from = ZOPFLI_MASTER_BLOCK_SIZE;
			let mut idx = costs.len() - 1;
			while 0 != from && 0 != idx {
				let v = costs[idx].1;
				if ! v.is_zero() && (v as usize) <= idx {
					from -= 1;
					self.paths[from] = v;
					idx -= v as usize;
				}
				else { return Err(zopfli_error!()) }
			}

			Ok(&self.paths[from..])
		}
	}
}



/// # Join Length Distance.
const fn ld_join(length: LitLen, distance: u16) -> u32 {
	let [l1, l2] = (length as u16).to_le_bytes();
	let [d1, d2] = distance.to_le_bytes();
	u32::from_le_bytes([l1, l2, d1, d2])
}

#[expect(unsafe_code, reason = "For transmute.")]
/// # Split Length Distance.
const fn ld_split(ld: u32) -> (LitLen, u16) {
	let [l1, l2, d1, d2] = ld.to_le_bytes();
	(
		// Safety: we're just undoing the work of ld_join, which had a valid
		// LitLen to start with.
		unsafe { std::mem::transmute::<u16, LitLen>(u16::from_le_bytes([l1, l2])) },
		u16::from_le_bytes([d1, d2]),
	)
}

/// # Max Sublength.
///
/// Return the maximum sublength length for a given cache chunk.
///
/// Each three-byte cache-entry has its length recorded in the first byte; the
/// last such entry holds the maximum.
const fn max_sublen(slice: &[u8; SUBLEN_CACHED_LEN]) -> LitLen {
	// If the first chunk has no distance, assume a zero length.
	if slice[1] == 0 && slice[2] == 0 { LitLen::L000 }
	// Otherwise the "max" is stored as the first value of the last chunk.
	// Since lengths are stored `-3`, we have to add three back to the stored
	// value to make it a real length.
	else { LitLen::from_packed_u8(slice[SUBLEN_CACHED_LEN - 3]) }
}

/// # Write Sublength.
///
/// Fill the provided sublength slice with data from the cache.
fn write_sublen(src: &[u8; SUBLEN_CACHED_LEN], dst: &mut [u16; SUBLEN_LEN]) {
	let maxlength = max_sublen(src);
	let mut old = 0;
	for chunk in src.chunks_exact(3) {
		let length = LitLen::from_packed_u8(chunk[0]);
		if old <= (length as usize) {
			let value = u16::from_le_bytes([chunk[1], chunk[2]]);
			dst[old..=length as usize].fill(value);
		}
		if (length as u16) >= (maxlength as u16) { return; }
		old = (length as usize) + 1;
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_split_join() {
		// Simple split test.
		let (len, dist) = ld_split(DEFAULT_LD);
		assert!(matches!(len, LitLen::L001));
		assert_eq!(dist, 0);

		// Joining should get us back where we started.
		assert_eq!(DEFAULT_LD, ld_join(len, dist));
	}

	#[test]
	fn t_split_mask() {
		// What we expect our masks to look like.
		const fn split_cache_mask(pos: usize) -> u8 {
			match pos % 8 {
				0 => 0b0000_0001,
				1 => 0b0000_0010,
				2 => 0b0000_0100,
				3 => 0b0000_1000,
				4 => 0b0001_0000,
				5 => 0b0010_0000,
				6 => 0b0100_0000,
				_ => 0b1000_0000,
			}
		}

		for pos in 0..255_usize {
			let mask: u8 = 1 << (pos % 8);
			assert_eq!(mask, split_cache_mask(pos));
		}
	}

	#[test]
	fn t_split_cache() {
		let mut cache = SplitCache {
			set: [0_u8; SPLIT_CACHE_LEN],
		};

		// Check that positions are false to start, true after set.
		for i in 0..ZOPFLI_MASTER_BLOCK_SIZE {
			assert!(cache.is_unset(i));
			cache.set(i);
			assert!(! cache.is_unset(i));
		}

		// Everything should be set now.
		assert!(cache.set.iter().all(|&b| b == u8::MAX));

		// If we initialize with a small value, only those bits should be
		// affected.
		cache.init(ZopfliRange::new(0, 32).unwrap());
		assert_eq!(cache.set[0], 0);
		assert_eq!(cache.set[1], 0);
		assert_eq!(cache.set[2], 0);
		assert_eq!(cache.set[3], 0);
		assert_eq!(cache.set[4], u8::MAX);
	}
}
