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

use std::os::raw::c_uint;
use super::ffi::EncodedImage;
use super::lodepng::{
	DecodedImage,
	LodePNGColorType,
	LodePNGFilterStrategy,
	LodePNGState,
};



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
	if count.is_null() || bitlengths.is_null() { return; }
	let count: &[usize] = unsafe { std::slice::from_raw_parts(count, n) };
	let bitlengths: &mut [f64] = unsafe { std::slice::from_raw_parts_mut(bitlengths, n) };

	// Sum the counts and log some shit.
	let sum = count.iter().copied().sum::<usize>();
	let log2sum =
		if sum == 0 { (n as f64).log2() }
		else { (sum as f64).log2() };

	for (&c, b) in count.iter().zip(bitlengths.iter_mut()) {
		// If the count is zero, give it the cost as if it were one since it
		// is being requested anyway.
		if c == 0 { *b = log2sum; }
		else { *b = (log2sum - (c as f64).log2()).max(0.0); }
	}
}

#[no_mangle]
#[allow(unsafe_code)]
#[inline]
/// # Zopfli Lengths to Symbols (`0..=7`).
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
pub(crate) extern "C" fn ZopfliLengthsToSymbols15(
	lengths: *const c_uint,
	n: usize,
	symbols: *mut c_uint,
) {
	zopfli_lengths_to_symbols::<16>(lengths, n, symbols);
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
	if lengths.is_null() || symbols.is_null() { return; }
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
