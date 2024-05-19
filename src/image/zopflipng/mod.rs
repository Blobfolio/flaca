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
pub(crate) use lz77::LZ77Store;
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
	DeflateSym,
	DISTANCE_BITS,
	DISTANCE_SYMBOLS,
	DISTANCE_VALUES,
	Dsym,
	LENGTH_SYMBOLS_BITS_VALUES,
	LitLen,
	Lsym,
};



/// # Fixed Trees (for extern).
const FIXED_TREE_LL: [DeflateSym; 288] = [
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
];
const FIXED_TREE_D: [DeflateSym; 32] = [DeflateSym::D05; 32];
const ZOPFLI_NUM_LL: usize = FIXED_TREE_LL.len();
const ZOPFLI_NUM_D: usize = FIXED_TREE_D.len();

// This is the biggest chunk-o-data that can be passed to deflate.
pub(super) const ZOPFLI_MASTER_BLOCK_SIZE: usize = 1_000_000;

// The matchable range.
const ZOPFLI_MIN_MATCH: usize = 3;
const ZOPFLI_MAX_MATCH: usize = 258;

/// # Length of Sublength Array.
///
/// This is hardcoded in `squeeze.c`.
const SUBLEN_LEN: usize = ZOPFLI_MAX_MATCH + 1;



#[allow(clippy::inline_always)]
#[inline(always)]
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



#[allow(clippy::inline_always)]
#[inline(always)]
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

#[inline(never)]
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

#[inline]
#[allow(unsafe_code)]
/// # Split Array.
///
/// Take a sized slice out of the collection, or die trying.
///
/// This is equivalent to `slice.get(rng).and_then(TryInto::try_into)`, but
/// less ugly!
const fn sized_slice<T, const N: usize>(slice: &[T], idx: usize)
-> Result<&[T; N], ZopfliError> {
	if idx + N <= slice.len() {
		unsafe {
			// Safety: the subslice is in range.
			Ok(&*(slice.as_ptr().add(idx).cast()))
		}
	}
	else { Err(zopfli_error!()) }
}
