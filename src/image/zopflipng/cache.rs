/*!
# Flaca: Zopflipng Caches.

This module defines the longest match and squeeze cache structures, and hosts
the thread-local LMC static.
*/

use super::{
	SUBLEN_LEN,
	zopfli_error,
	ZOPFLI_MAX_MATCH,
	ZOPFLI_MIN_MATCH,
	ZopfliError,
};



/// # Default Length (1) and Distance (0).
///
/// Length and distance are always fetched/stored together, so are grouped into
/// a single value to reduce indexing/bounds overhead.
const DEFAULT_LD: (u16, u16) = (1, 0);

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
	ld: Vec<(u16, u16)>,
	sublen: Vec<u8>,
}

impl MatchCache {
	/// # New.
	pub(super) const fn new() -> Self {
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
		limit: &mut usize,
		sublen: &mut Option<&mut [u16; SUBLEN_LEN]>,
		distance: &mut u16,
		length: &mut u16,
	) -> Result<bool, ZopfliError> {
		// One sanity check to rule them all.
		if pos >= self.ld.len() { return Err(zopfli_error!()); }
		let cache_sublen: &[u8; SUBLEN_CACHED_LEN] = self.sublen.get(SUBLEN_CACHED_LEN * pos..SUBLEN_CACHED_LEN * pos + SUBLEN_CACHED_LEN)
			.and_then(|s| s.try_into().ok())
			.ok_or(zopfli_error!())?;

		// If we have no distance, we have no cache.
		let (cache_len, cache_dist) = self.ld[pos];
		if cache_len != 0 && cache_dist == 0 { return Ok(false); }

		// Find the max sublength once, if ever.
		let maxlength =
			if sublen.is_none() { 0 }
			else { max_sublen(cache_sublen) };

		// Proceed if our cached length or max sublength are under the limit.
		if
			*limit == ZOPFLI_MAX_MATCH ||
			usize::from(cache_len) <= *limit ||
			(sublen.is_some() && maxlength >= *limit)
		{
			// Update length and distance if the sublength pointer is null or
			// the cached sublength is bigger than the cached length.
			if sublen.is_none() || usize::from(cache_len) <= maxlength {
				// Cap the length.
				*length = cache_len;
				if usize::from(*length) > *limit { *length = *limit as u16; }

				// Set the distance from the sublength cache.
				if let Some(s) = sublen {
					// Pull the sublength from cache and pull the distance from
					// that.
					if 3 <= *length { write_sublen(cache_sublen, s); }
					*distance = s[usize::from(*length)];

					// Sanity check: make sure the sublength distance at length
					// matches the redundantly-cached distance.
					if
						*limit == ZOPFLI_MAX_MATCH &&
						usize::from(*length) >= ZOPFLI_MIN_MATCH &&
						*distance != cache_dist
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
			*limit = usize::from(cache_len);
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
		length: u16,
	) -> Result<(), ZopfliError> {
		match self.ld.get(pos).copied() {
			// If the current value is the default, let's proceed!
			Some(DEFAULT_LD) => {},
			// If the current value is something else and legit, abort happy.
			Some((l, d)) if l == 0 || d != 0 => return Ok(()),
			// Otherwise abort sad!
			_ => return Err(zopfli_error!()),
		}

		// The sublength isn't cacheable, but that fact is itself worth
		// caching!
		if usize::from(length) < ZOPFLI_MIN_MATCH {
			self.ld[pos] = (0, 0);
			return Ok(());
		}

		// Reslice it to the (inclusive) length, ignoring the first 3 entries
		// since they're below the minimum give-a-shittable limit. Note that
		// without them, each index can be represented (and stored) as a u8.
		let slice = sublen.get(ZOPFLI_MIN_MATCH..=usize::from(length))
			.ok_or(zopfli_error!())?;

		// Save the length/distance bit.
		if distance == 0 { return Err(zopfli_error!()); }
		self.ld[pos] = (length, distance);

		// The cache gets written three bytes at a time; this iterator will
		// help us eliminate the bounds checks we'd otherwise run into.
		let mut dst = self.sublen.chunks_exact_mut(3)
			.skip(ZOPFLI_CACHE_LENGTH * pos)
			.take(ZOPFLI_CACHE_LENGTH);

		// Start by writing all mismatched pairs, up to the limit.
		for (i, pair) in (0_u8..).zip(slice.windows(2)) {
			if pair[0] != pair[1] {
				let Some([d0, d1, d2]) = dst.next() else { return Ok(()); };
				*d0 = i;
				[*d1, *d2] = pair[0].to_le_bytes();
			}
		}

		// The final value is implicitly "mismatched"; if we haven't hit the
		// limit we should write it too.
		if let Some([d0, d1, d2]) = dst.next() {
			*d0 = (length - 3) as u8;
			[*d1, *d2] = slice[slice.len() - 1].to_le_bytes();

			// If we're still below the limit, copy (only) the length to the
			// last slot to simplify any subsequent max_length lookups.
			if let Some([d0, _, _]) = dst.last() { *d0 = (length - 3) as u8; }
		}

		Ok(())
	}
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
