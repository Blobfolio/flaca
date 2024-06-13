/*!
# Flapfli: Longest Match Cache.

The LMC is used to eleviate some of the burden that would otherwise result from
calling `ZopfliHash::find` a hundred million times in a row. Haha.
*/

use std::{
	alloc::{
		alloc,
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
	LitLen,
	SUBLEN_LEN,
	zopfli_error,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZOPFLI_MIN_MATCH,
	ZopfliError,
};



/// # Default Length (1) and Distance (0).
///
/// Length and distance are always fetched/stored together, so are grouped into
/// a single value to reduce indexing/bounds overhead.
const DEFAULT_LD: u32 = u32::from_le_bytes([1, 0, 0, 0]);

/// # Sublength Cache Entries.
const ZOPFLI_CACHE_LENGTH: usize = 8;

/// # Sublength Cache Total Length.
///
/// Each entry uses three bytes, so the total size is…
const SUBLEN_CACHED_LEN: usize = ZOPFLI_CACHE_LENGTH * 3;



/// # Longest Match Cache.
///
/// This structure holds cached length/distance details for individual
/// sublengths. Its memory usage is no joke, but the performance savings more
/// than make up for it.
pub(crate) struct MatchCache {
	ld: [u32; ZOPFLI_MASTER_BLOCK_SIZE],
	sublen: [[u8; SUBLEN_CACHED_LEN]; ZOPFLI_MASTER_BLOCK_SIZE],
}

impl MatchCache {
	#[allow(unsafe_code)]
	/// # New.
	///
	/// Arrays holding a million elements are obviously less than ideal, but
	/// because these are referenced repeatedly with different sub-slice sizes,
	/// it is much better for performance than vectors that have to be
	/// continuously resized/reallocated.
	///
	/// Still, these are too big for the stack, so we're initializing them via
	/// raw pointers and jamming them straight into a `Box`.
	pub(super) fn new() -> Box<Self> {
		// Reserve the space.
		const LAYOUT: Layout = Layout::new::<MatchCache>();
		let out = NonNull::new(unsafe { alloc(LAYOUT).cast() })
			.unwrap_or_else(|| handle_alloc_error(LAYOUT));
		let ptr: *mut Self = out.as_ptr();

		unsafe {
			// The arrays can be zero-filled to start with; they'll get reset
			// prior to use anyway.
			addr_of_mut!((*ptr).ld).write_bytes(0, 1);
			addr_of_mut!((*ptr).sublen).write_bytes(0, 1);

			// All set!
			Box::from_raw(ptr)
		}
	}

	/// # Initialize.
	///
	/// This resizes the cache buffers and resets their values to their default
	/// states — one for length, zero for everything else.
	///
	/// Because this is a shared buffer, allocations persist for the duration
	/// of the program run so they can be reused.
	pub(crate) fn init(&mut self, mut blocksize: usize) {
		// Lodepng will never pass along more than ZOPFLI_MASTER_BLOCK_SIZE
		// bytes, but this lets the compiler know we won't go over.
		if ZOPFLI_MASTER_BLOCK_SIZE < blocksize {
			blocksize = ZOPFLI_MASTER_BLOCK_SIZE;
		}

		// Lengths default to one, everything else to zero.
		self.ld[..blocksize].fill(DEFAULT_LD);
		self.sublen[..blocksize].fill([0; SUBLEN_CACHED_LEN]);
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Find Match.
	///
	/// Find the sublength, distance, and length from cache, if possible.
	///
	/// Values are written directly to the passed arguments. A bool is returned
	/// to indicate whether or not the find was successful.
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
		let cache_sublen: &[u8; SUBLEN_CACHED_LEN] = &self.sublen[pos];

		// Find the max sublength once, if ever.
		let maxlength =
			if sublen.is_none() { 0 }
			else { max_sublen(cache_sublen) };

		// Proceed if our cached length or max sublength are under the limit.
		if
			limit.is_max() ||
			(cache_len as u16) <= (*limit as u16) ||
			(sublen.is_some() && maxlength >= (*limit as usize))
		{
			// Update length and distance if the sublength pointer is null or
			// the cached sublength is bigger than the cached length.
			if sublen.is_none() || (cache_len as usize) <= maxlength {
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

	#[allow(clippy::cast_possible_truncation)]
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
		let mut dst = self.sublen[pos].chunks_exact_mut(3);

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
			*d0 = (length as u16 - 3) as u8;
			[*d1, *d2] = slice[slice.len() - 1].to_le_bytes();

			// If we're still below the limit, copy (only) the length to the
			// last slot to simplify any subsequent max_length lookups.
			if let Some([d0, _, _]) = dst.last() { *d0 = (length as u16 - 3) as u8; }
		}

		Ok(())
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
/// a thread-local static so will be reused as many times as needed.
pub(crate) struct SqueezeCache {
	costs: [(f32, LitLen); ZOPFLI_MASTER_BLOCK_SIZE + 1],
	paths: [LitLen; ZOPFLI_MASTER_BLOCK_SIZE],
	costs_len: Cell<usize>,
}

impl SqueezeCache {
	#[allow(unsafe_code)]
	/// # New (Boxed) Instance.
	///
	/// Arrays holding a million+ elements is obviously less than ideal, but
	/// because these are referenced repeatedly with different sub-slice sizes,
	/// it is much better for performance than vectors that have to be
	/// continuously resized/reallocated.
	///
	/// Still, these are too big for the stack, so we're initializing them via
	/// raw pointers and jamming them straight into a `Box`.
	pub(crate) fn new() -> Box<Self> {
		// Reserve the space.
		const LAYOUT: Layout = Layout::new::<SqueezeCache>();
		let out = NonNull::new(unsafe { alloc(LAYOUT).cast() })
			.unwrap_or_else(|| handle_alloc_error(LAYOUT));
		let ptr: *mut Self = out.as_ptr();

		unsafe {
			// The arrays can be zero-filled to start with; they'll be reset
			// or overwritten before use anyway.
			addr_of_mut!((*ptr).costs).write_bytes(0, 1);
			addr_of_mut!((*ptr).paths).write_bytes(0, 1);

			// Zero works equally well for the initial length, especially
			// because it's true! Haha.
			addr_of_mut!((*ptr).costs_len).write(Cell::new(0));

			// All set!
			Box::from_raw(ptr)
		}
	}

	/// # Resize Costs.
	///
	/// This sets the internal costs length to match the desired blocksize, but
	/// does _not_ reset their values. (Unlike the LMC, which more or less
	/// persists for the duration of a given block, costs are calculated and
	/// discarded and recalculated and discarded… several times.)
	pub(crate) fn resize_costs(&self, blocksize: usize) {
		self.costs_len.set(blocksize);
	}

	/// # Reset Costs.
	///
	/// Reset and return a mutable slice of costs, sized according to the last
	/// `resize_costs` call.
	///
	/// Note that only the costs themselves are reset; the lengths and paths
	/// are dealt with _in situ_ during crunching (without being read).
	pub(crate) fn reset_costs(&mut self) -> &mut [(f32, LitLen)] {
		let costs = self.costs.get_mut(..self.costs_len.get()).unwrap_or(&mut []);
		if ! costs.is_empty() {
			// The first cost needs to be zero; the rest need to be infinity.
			costs[0].0 = 0.0;
			for c in costs.iter_mut().skip(1) { c.0 = f32::INFINITY; }
		}
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

#[allow(unsafe_code)]
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
/// Return the maximum sublength length for a given chunk.
const fn max_sublen(slice: &[u8; SUBLEN_CACHED_LEN]) -> usize {
	// If the first chunk has no distance, assume a zero length.
	if slice[1] == 0 && slice[2] == 0 { 0 }
	// Otherwise the "max" is stored as the first value of the last chunk.
	else { slice[SUBLEN_CACHED_LEN - 3] as usize + 3 }
}

/// # Write Sublength.
///
/// Fill the provided sublength slice with data from the cache.
fn write_sublen(src: &[u8; SUBLEN_CACHED_LEN], dst: &mut [u16; SUBLEN_LEN]) {
	let maxlength = max_sublen(src);
	let mut old = 0;
	for chunk in src.chunks_exact(3) {
		let length = usize::from(chunk[0]) + ZOPFLI_MIN_MATCH;
		if old <= length {
			let value = u16::from_le_bytes([chunk[1], chunk[2]]);
			dst[old..=length].fill(value);
		}
		if length == maxlength { return; }
		old = length + 1;
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
}
