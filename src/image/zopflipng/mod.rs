/*!
# Flaca: Zopflipng!

The `optimize` method in this module emulates the behaviors of the zopflipng
CLI tool when called with:

```bash
zopflipng -m <input> <output>
```

As Google is no longer maintaining the original zopfli project, all relevant
functionality supporting the above has been rewritten and ported to Rust,
resulting in code that is safer, (slightly) saner, and ultimately more
performant.
*/

mod blocks;
mod cache;
mod error;
mod hash;
mod kat;
mod lz77;
mod stats;
mod symbols;

pub(crate) use blocks::{
	deflate_part,
	SplitPoints,
};
use cache::MatchCache;
use error::{
	zopfli_error,
	ZopfliError,
};
pub(crate) use hash::ZopfliState;
use lz77::LZ77Store;
use kat::zopfli_length_limited_code_lengths;
use super::{
	ffi::EncodedImage,
	lodepng::{
		DecodedImage,
		LodePNGColorType,
		LodePNGFilterStrategy,
		LodePNGState,
		ZopfliOut,
	},
};
use symbols::{
	DISTANCE_BITS,
	DISTANCE_SYMBOLS,
	DISTANCE_VALUES,
	Dsym,
	LENGTH_SYMBOLS_BITS_VALUES,
	LitLen,
	Lsym,
};



/// # Fixed Trees (for extern).
const FIXED_TREE_LL: [u32; 288] = [
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
const FIXED_TREE_D: [u32; 32] = [5; 32];
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
/// This updates the symbol array given the corresponding lengths.
fn zopfli_lengths_to_symbols<const MAXBITS: usize, const SIZE: usize>(
	lengths: &[u32; SIZE],
	symbols: &mut [u32; SIZE],
) {
	// Count up the codes by code length.
	let mut counts: [u32; MAXBITS] = [0; MAXBITS];
	for l in lengths {
		let l = *l as usize;
		if l < MAXBITS { counts[l] += 1; }
		else { return; }
	}

	// Find the numerical value of the smallest code for each code length.
	counts[0] = 0;
	let mut code = 0;
	let mut next_code: [u32; MAXBITS] = [0; MAXBITS];
	for i in 1..MAXBITS {
		code = (code + counts[i - 1]) << 1;
		next_code[i] = code;
	}

	// Update the symbols accordingly.
	for (s, l) in symbols.iter_mut().zip(lengths.iter().copied()) {
		if l == 0 { *s = 0; }
		else {
			// Safety: all lengths were tested to be < MAXBITS a few lines up.
			debug_assert!((l as usize) < MAXBITS);
			*s = unsafe { *next_code.get_unchecked(l as usize) };
			next_code[l as usize] += 1;
		}
	}
}
