/*!
# Flaca: Zopflipng

The `optimize` method in this module emulates the behaviors of the zopflipng
CLI tool when called with:

```bash
zopflipng -m <input> <output>
```

This no longer links to `libzopflipng` itself, but instead reimplements its
functionality.
*/

mod cache;
mod hash;
mod kat;

use cache::{
	CACHE,
	SQUEEZE,
};
use std::os::raw::c_uint;
use super::ffi::EncodedImage;
use super::lodepng::{
	DecodedImage,
	LodePNGColorType,
	LodePNGFilterStrategy,
	LodePNGState,
	SymbolStats,
	ZopfliStoreLitLenDist,
	ZopfliLZ77Store,
};



/// # Fixed Trees (for extern).
const FIXED_TREE_LL: [c_uint; 288] = [
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8,
	8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 9, 9, 9, 9, 9, 9,
	9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
	9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
	9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
	9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
	9, 9, 9, 9, 9, 9, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7,
	7, 7, 7, 7, 7, 8, 8, 8, 8, 8, 8, 8, 8
];
const FIXED_TREE_D: [c_uint; 32] = [5; 32];
const ZOPFLI_NUM_LL: usize = FIXED_TREE_LL.len();
const ZOPFLI_NUM_D: usize = FIXED_TREE_D.len();

const ZOPFLI_MAX_MATCH: usize = 258;
const ZOPFLI_MIN_MATCH: usize = 3;

/// # Length of Sublength Array.
///
/// This is hardcoded in `squeeze.c`.
const SUBLEN_LEN: usize = ZOPFLI_MAX_MATCH + 1;



/// # Optimize!
///
/// This will attempt to losslessly recompress the source PNG with the
/// strongest Zopfli filter strategy, and return a new PNG image if the result
/// is smaller than the original.
///
/// Note: 16-bit transformations are not lossless; such images will have their
/// bit depths reduced to a more typical 8 bits.
pub(super) fn optimize(src: &[u8]) -> Option<EncodedImage<usize>> {
	let mut dec = LodePNGState::default();
	let img = dec.decode(src)?;

	// Encode!
	let strategy = best_strategy(&dec, &img);
	let out = encode(&dec, &img, strategy, true)?;

	// Return it if better and nonzero!
	if out.size < src.len() { Some(out) }
	else { None }
}



#[no_mangle]
#[allow(unsafe_code)]
/// # Write Fixed Tree.
///
/// This is a rewrite of the original `deflate.c` method.
///
/// Note: the magic lengths (288 and 32) correspond to `ZOPFLI_NUM_LL` and
/// `ZOPFLI_NUM_D`, defined in `util.h`.
pub(crate) const extern "C" fn GetFixedTree(ll_lengths: *mut c_uint, d_lengths: *mut c_uint) {
	unsafe {
		std::ptr::copy_nonoverlapping(FIXED_TREE_LL.as_ptr(), ll_lengths, ZOPFLI_NUM_LL);
		std::ptr::copy_nonoverlapping(FIXED_TREE_D.as_ptr(), d_lengths, ZOPFLI_NUM_D);
	}
}

#[no_mangle]
#[allow(unsafe_code, clippy::integer_division, clippy::cast_sign_loss)]
/// # Optimize Huffman RLE Compression.
///
/// This is a rewrite of the original `deflate.c` method.
pub(crate) extern "C" fn OptimizeHuffmanForRle(length: usize, counts: *mut usize) {
	// Convert counts to a proper slice with trailing zeroes trimmed.
	let ptr = counts;
	let mut counts: &mut [usize] = unsafe { std::slice::from_raw_parts_mut(counts, length) };
	while let [ rest @ .., 0 ] = counts { counts = rest; }
	if counts.is_empty() { return; }

	// Find collapseable ranges!
	let mut stride = 0;
	let mut scratch = counts[0];
	let mut sum = 0;
	let four = counts.len().saturating_sub(3);
	for (i, (&count, good)) in counts.iter().zip(GoodForRle::new(counts)).enumerate() {
		// Time to reset (and maybe collapse).
		if good || count.abs_diff(scratch) >= 4 {
			// Collapse the stride if it is as least four and contained
			// something non-zero.
			if sum != 0 && stride >= 4 {
				let v = ((sum + stride / 2) / stride).max(1);
				// Safety: this is a very un-Rust thing to do, but we're only
				// modifying values after-the-fact; the current and future
				// data remains as it was.
				unsafe {
					for j in i - stride..i { ptr.add(j).write(v); }
				}
			}

			// Reset!
			stride = 0;
			sum = 0;

			// If we have at least four remaining values (including the
			// current), take a sort of weighted average of them.
			if i < four {
				scratch = (
					unsafe { *counts.get_unchecked(i + 3) } +
					counts[i + 2] +
					counts[i + 1] +
					count +
					2
				) / 4;
			}
			// Otherwise just use the current value.
			else { scratch = count; }
		}

		stride += 1;
		sum += count;
	}

	// Collapse the trailing stride, if any.
	if sum != 0 && stride >= 4 {
		let v = ((sum + stride / 2) / stride).max(1);
		let len = counts.len();
		counts[len - stride..].fill(v);
	}
}

#[no_mangle]
#[allow(unsafe_code)]
/// # Patch Buggy Distance Codes.
///
/// Ensure there are at least two distance codes to avoid issues with buggy
/// decoders.
///
/// Note: `d_lengths` has a fixed length of 32.
pub(crate) extern "C" fn PatchDistanceCodesForBuggyDecoders(d_lengths: *mut c_uint) {
	let mut one: Option<usize> = None;
	for i in 0..30 {
		// We have (at least) two non-zero entries; no patching needed!
		if 0 != unsafe { *d_lengths.add(i) } && one.replace(i).is_some() {
			return;
		}
	}

	match one {
		// The first entry had a code, so patch the second to give us two.
		Some(0) => unsafe { d_lengths.add(1).write(1); },
		// Patch the first entry to give us two.
		Some(_) => unsafe { d_lengths.write(1); },
		// There were no codes, so let's just patch the first two.
		None => unsafe { d_lengths.write_bytes(1, 2); },
	}
}

#[no_mangle]
#[allow(unsafe_code, clippy::cast_precision_loss)]
/// # Zopfli Claculate Entropy.
///
/// This is a rewrite of the original `tree.c` method.
pub(crate) extern "C" fn ZopfliCalculateEntropy(
	count: *const usize,
	n: usize,
	bitlengths: *mut f64
) {
	// Turn the pointers into slices.
	if n == 0 { return; }
	let count: &[usize] = unsafe { std::slice::from_raw_parts(count, n) };

	// Sum the counts and log some shit.
	let sum = count.iter().copied().sum::<usize>();

	// If there are no counts, every value has the same cost.
	if sum == 0 {
		let log2sum = (n as f64).log2();
		unsafe {
			for i in 0..n { bitlengths.add(i).write(log2sum); }
		}
	}
	// Otherwise each gets its own fractional cost.
	else {
		let log2sum = (sum as f64).log2();

		for (i, &c) in count.iter().enumerate() {
			// Even zeroes get a cost because they were requested.
			if c == 0 {
				unsafe { bitlengths.add(i).write(log2sum); }
			}
			else {
				// Floating point math sucks; make sure it doesn't magically
				// drop below zero.
				let mut v = log2sum - (c as f64).log2();
				if v.is_sign_negative() { v = 0.0; }
				unsafe { bitlengths.add(i).write(v); }
			}
		}
	}
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Zopfli Lengths to Symbols (`0..=7`).
///
/// This extern is a convenience wrapper for calling
/// `zopfli_lengths_to_symbols` with a constant length of 8.
pub(crate) extern "C" fn ZopfliLengthsToSymbols7(
	lengths: *const c_uint,
	n: usize,
	symbols: *mut c_uint,
) {
	zopfli_lengths_to_symbols::<8>(lengths, n, symbols);
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Zopfli Lengths to Symbols (`0..=15`).
///
/// This extern is a convenience wrapper for calling
/// `zopfli_lengths_to_symbols` with a constant length of 16.
pub(crate) extern "C" fn ZopfliLengthsToSymbols15(
	lengths: *const c_uint,
	n: usize,
	symbols: *mut c_uint,
) {
	zopfli_lengths_to_symbols::<16>(lengths, n, symbols);
}



/// # RLE-Optimized Stretches.
///
/// This iterator yields a boolean value for each entry of the source slice,
/// `true` for distance codes in a sequence of 5+ zeroes or 7+ (identical)
/// non-zeroes, `false` otherwise.
///
/// (Such ranges are already RLE-optimal.)
struct GoodForRle<'a> {
	counts: &'a [usize],
	good: usize,
	bad: usize,
}

impl<'a> GoodForRle<'a> {
	/// # New Instance.
	const fn new(counts: &'a [usize]) -> Self {
		Self { counts, good: 0, bad: 0 }
	}
}

impl<'a> Iterator for GoodForRle<'a> {
	type Item = bool;

	fn next(&mut self) -> Option<Self::Item> {
		// Return good or bad values from the buffer.
		if self.good != 0 {
			self.good -= 1;
			return Some(true);
		}
		if self.bad != 0 {
			self.bad -= 1;
			return Some(false);
		}

		// If the slice is empty, we're done!
		if self.counts.is_empty() { return None; }

		// See how many times the next entry is repeated, if at all, shortening
		// the slice accordingly.
		let scratch = self.counts[0];
		let mut stride = 0;
		while let [count, rest @ ..] = self.counts {
			// Note the reptition and circle back around.
			if *count == scratch {
				stride += 1;
				self.counts = rest;
			}
			// We had an optimal stretch.
			else if stride >= 5 && (scratch == 0 || stride >= 7) {
				self.good = stride - 1;
				return Some(true);
			}
			// We had a non-optimal stretch.
			else {
				self.bad = stride - 1;
				return Some(false);
			}
		}

		// Finish up by qualifying the dangling stride as optimal or not.
		if stride >= 5 && (scratch == 0 || stride >= 7) {
			self.good = stride - 1;
			Some(true)
		}
		else {
			self.bad = stride - 1;
			Some(false)
		}
	}

	fn size_hint(&self) -> (usize, Option<usize>) {
		let len = self.len();
		(len, Some(len))
	}
}

impl<'a> ExactSizeIterator for GoodForRle<'a> {
	fn len(&self) -> usize { self.good + self.bad + self.counts.len() }
}



/// # Best Strategy.
///
/// This attempts to find the best filtering strategy for the image by trying
/// all of them in fast mode, and picking whichever produces the smallest
/// output.
fn best_strategy(dec: &LodePNGState, img: &DecodedImage) -> LodePNGFilterStrategy {
	[
		LodePNGFilterStrategy::LFS_ZERO,
		LodePNGFilterStrategy::LFS_ONE,
		LodePNGFilterStrategy::LFS_TWO,
		LodePNGFilterStrategy::LFS_THREE,
		LodePNGFilterStrategy::LFS_FOUR,
		LodePNGFilterStrategy::LFS_MINSUM,
		LodePNGFilterStrategy::LFS_ENTROPY,
		LodePNGFilterStrategy::LFS_BRUTE_FORCE,
	]
		.into_iter()
		.filter_map(|s| encode(dec, img, s, false).map(|out| (out.size, s)))
		.min_by(|a, b| a.0.cmp(&b.0))
		.map_or(LodePNGFilterStrategy::LFS_ZERO, |(_, s)| s)
}

/// # Apply Optimizations.
///
/// This attempts to re-encode an image using the provided filter strategy,
/// returning an `EncodedImage` object if it all works out.
fn encode(
	dec: &LodePNGState,
	img: &DecodedImage,
	strategy: LodePNGFilterStrategy,
	slow: bool,
) -> Option<EncodedImage<usize>> {
	// Encode and write to the buffer if it worked.
	let mut enc = LodePNGState::encoder(dec, strategy, slow)?;
	let out = enc.encode(img)?;

	// We might be able to save a couple bytes by nuking the palette if the
	// image is already really small.
	if
		out.size < 4096 &&
		LodePNGColorType::LCT_PALETTE.is_match(&out) &&
		enc.prepare_encoder_small(img)
	{
		if let Some(out2) = enc.encode(img) {
			if out2.size < out.size {
				return Some(out2);
			}
		}
	}

	Some(out)
}

#[allow(unsafe_code)]
/// # Zopfli Lengths to Symbols.
///
/// This is a rewrite of the method `ZopfliLengthsToSymbols` from `tree.c`.
fn zopfli_lengths_to_symbols<const MAXBITS: usize>(
	lengths: *const c_uint,
	n: usize,
	symbols: *mut c_uint,
) {
	// Convert lengths and symbols into usable slices, and maxbits into usize.
	if n == 0 { return; }
	let lengths: &[c_uint] = unsafe { std::slice::from_raw_parts(lengths, n) };
	let symbols: &mut [c_uint] = unsafe { std::slice::from_raw_parts_mut(symbols, n) };

	// Count up the codes by code length.
	let mut counts: [c_uint; MAXBITS] = [0; MAXBITS];
	for l in lengths {
		let l = *l as usize;
		if l < MAXBITS { counts[l] += 1; }
		else { return; }
	}

	// Find the numerical value of the smallest code for each code length.
	counts[0] = 0;
	let mut code = 0;
	let mut next_code: [c_uint; MAXBITS] = [0; MAXBITS];
	for i in 1..MAXBITS {
		code = (code + counts[i - 1]) << 1;
		next_code[i] = code;
	}

	// Update the symbols accordingly.
	for (s, l) in symbols.iter_mut().zip(lengths.iter()) {
		let l = *l as usize;
		if l == 0 { *s = 0; }
		else {
			*s = next_code[l];
			next_code[l] += 1;
		}
	}
}



#[cfg(test)]
mod test {
	use super::*;

	#[test]
	fn t_good_for_rle() {
		const COUNTS1: &[usize] = &[196, 23, 10, 12, 5, 4, 1, 23, 8, 2, 6, 5, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 47, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 8, 1, 1, 1, 0, 4, 7, 2, 0, 1, 10, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 42, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 44, 284, 1, 15, 21, 0, 55, 0, 19, 0, 1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 320, 12, 0, 0, 17, 3, 0, 3, 2];
		const COUNTS2: &[usize] = &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 122, 0, 288, 11, 41, 6, 5, 2, 0, 0, 0, 1];
		const COUNTS3: &[usize] = &[201, 24, 10, 12, 5, 4, 1, 24, 8, 2, 6, 4, 0, 0, 0, 29, 5, 0, 0, 4, 4, 1, 0, 5, 2, 0, 0, 1, 4, 0, 1, 34, 10, 5, 7, 2, 1, 2, 0, 0, 3, 2, 5, 0, 1, 0, 0, 4, 2, 1, 0, 0, 1, 1, 0, 1, 1, 2, 0, 1, 4, 1, 5, 47, 13, 0, 5, 3, 1, 2, 0, 4, 0, 1, 6, 3, 0, 0, 0, 1, 3, 2, 2, 1, 4, 6, 0, 5, 0, 0, 1, 0, 0, 0, 1, 10, 4, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 1, 3, 1, 0, 0, 4, 0, 5, 49, 28, 3, 2, 5, 3, 0, 0, 1, 7, 0, 9, 1, 1, 1, 0, 4, 6, 2, 0, 1, 8, 0, 0, 2, 1, 0, 0, 1, 0, 0, 0, 7, 11, 4, 1, 1, 0, 3, 0, 1, 1, 1, 5, 1, 0, 0, 0, 4, 5, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 2, 0, 0, 2, 13, 27, 4, 1, 4, 1, 1, 0, 2, 2, 0, 0, 0, 3, 0, 0, 3, 8, 0, 0, 1, 0, 0, 0, 2, 1, 0, 0, 0, 1, 1, 1, 4, 24, 1, 4, 4, 2, 2, 0, 5, 6, 1, 1, 1, 1, 1, 0, 0, 44, 6, 3, 3, 3, 6, 0, 6, 30, 9, 10, 8, 33, 9, 46, 281, 1, 20, 3, 10, 59, 0, 4, 12, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 13, 318, 12, 0, 0, 21, 0, 0, 3, 2];

		for c in [COUNTS1, COUNTS2, COUNTS3] {
			// Make sure our ExactSizeness is working.
			let good = GoodForRle::new(c);
			assert_eq!(
				good.len(),
				c.len(),
				"GoodForRle iterator count does not match source.",
			);

			// And make sure that is the count we actually end up with.
			let good = good.collect::<Vec<bool>>();
			assert_eq!(
				good.len(),
				c.len(),
				"Collected GoodForRle iterator count does not match source.",
			);
		}
	}
}
