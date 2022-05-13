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
pub(super) fn optimize(src: &[u8]) -> Option<Vec<u8>> {
	let mut dec = LodePNGState::default();
	let img = dec.decode(src)?;

	// Encode!
	let strategy = best_strategy(&dec, &img);
	let out = optimize_slow(&dec, &img, strategy)?;

	// Return it if better and nonzero!
	if out.len() < src.len() { Some(out) }
	else { None }
}



/// # Best Strategy.
///
/// This attempts to find the best filtering strategy for the image by trying
/// all of them in fast mode, and picking whichever produces the smallest
/// output.
///
/// The lodepng `LFS_PREDEFINED` filter strategy is currently unsupported, but
/// isn't very common so shouldn't affect compression much one way or another.
fn best_strategy(dec: &LodePNGState, img: &DecodedImage) -> LodePNGFilterStrategy {
	[
		LodePNGFilterStrategy::LFS_ZERO,
		LodePNGFilterStrategy::LFS_ONE,
		LodePNGFilterStrategy::LFS_TWO,
		LodePNGFilterStrategy::LFS_THREE,
		LodePNGFilterStrategy::LFS_FOUR,
		LodePNGFilterStrategy::LFS_MINSUM,
		LodePNGFilterStrategy::LFS_ENTROPY,
	]
		.into_iter()
		.filter_map(|s| optimize_fast(dec, img, s).map(|size| (size, s)))
		.min_by(|a, b| a.0.cmp(&b.0))
		.map_or(LodePNGFilterStrategy::LFS_ZERO, |(_, s)| s)
}

/// # Test Optimizations.
///
/// This tests a given filter strategy (without the zopfli overhead) to get an
/// idea for the potential savings it would yield.
///
/// This will return the resulting size if it worked.
fn optimize_fast(
	dec: &LodePNGState,
	img: &DecodedImage,
	strategy: LodePNGFilterStrategy,
) -> Option<usize> {
	// Encode and write to the buffer if it worked.
	let mut enc = LodePNGState::encoder(dec, strategy, false)?;
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
				return Some(out2.size);
			}
		}
	}

	Some(out.size)
}

/// # Apply Optimizations.
///
/// This re-encodes the PNG source using the specified strategy, returning a
/// newly-allocated image if everything works out.
fn optimize_slow(
	dec: &LodePNGState,
	img: &DecodedImage,
	strategy: LodePNGFilterStrategy,
) -> Option<Vec<u8>> {
	// Encode and write to the buffer if it worked.
	let mut enc = LodePNGState::encoder(dec, strategy, true)?;
	enc.encode(img)
		.map(|out| {
			let out = out.to_vec();
			if out.len() < 4096 && LodePNGColorType::LCT_PALETTE.is_match(&out) {
				optimize_slow_small(img, out, &mut enc)
			}
			else { out }
		})
}

#[cold]
#[allow(clippy::cast_possible_truncation)]
/// # Apply Optimizations (Small).
///
/// For really small images, space can sometimes be saved by nuking the
/// palette and going with RGB/RGBA instead.
///
/// This will either return a new-new image or pass through the previously-
/// created new image if the trick doesn't work.
fn optimize_slow_small(img: &DecodedImage, mut buf: Vec<u8>, enc: &mut LodePNGState) -> Vec<u8> {
	if enc.prepare_encoder_small(img) {
		if let Some(out) = enc.encode(img) {
			if out.size < buf.len() {
				buf.truncate(out.size);
				buf.copy_from_slice(&out);
			}
		}
	}

	buf
}
