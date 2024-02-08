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
