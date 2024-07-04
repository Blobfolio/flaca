/*!
# Flapfli.

This library contains a (mostly) Rust port of [`zopflipng`](https://github.com/google/zopfli/),
heavily optimized flaca's specific use cases (hence "fla" + "pfli").
*/

#![deny(unsafe_code)]

#![warn(
	clippy::filetype_is_file,
	clippy::integer_division,
	clippy::needless_borrow,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::suboptimal_flops,
	clippy::unneeded_field_pattern,
	macro_use_extern_crate,
	missing_copy_implementations,
	missing_debug_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unreachable_pub,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![allow(
	clippy::module_name_repetitions,
	clippy::redundant_pub_crate,
)]

mod deflate;
mod ffi;
mod lodepng;
mod zopflipng;

use ffi::EncodedPNG;
use lodepng::{
	DecodedImage,
	LodePNGColorType,
	LodePNGFilterStrategy,
	LodePNGState,
};
use std::sync::atomic::AtomicU32;
use zopflipng::{
	deflate_part,
	reset_dynamic_length_cache,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliChunk,
	ZopfliState,
};



/// # Number of Zopfli Iterations.
///
/// A non-zero value indicates a fixed user preference (capped at `i32::MAX`,
/// though anything above a few thousand is usually terrible). If zero, the
/// number of iterations will vary by file size.
///
/// This is only actually written to once, if ever, but is atomic to make it
/// easier to read the value from within the callback. (That callback is Rust,
/// but called from C.)
pub static ZOPFLI_ITERATIONS: AtomicU32 = AtomicU32::new(0);

#[must_use]
/// # Optimize!
///
/// This will attempt to losslessly recompress the source PNG with the
/// strongest Zopfli filter strategy, and return a new PNG image if the result
/// is smaller than the original.
///
/// Note: 16-bit transformations are not lossless; such images will have their
/// bit depths reduced to a more typical 8 bits.
pub fn optimize(src: &[u8]) -> Option<EncodedPNG> {
	// Start by decoding the source.
	let mut dec = LodePNGState::default();
	let img = dec.decode(src)?;

	// Find the right strategy.
	let mut enc = LodePNGState::encoder(&dec)?;
	let mut out = EncodedPNG::new();
	let strategy = best_strategy(&img, &mut enc, &mut out);

	// Now re-re-encode with zopfli and the best strategy.
	enc.set_strategy(strategy);
	enc.set_zopfli();
	if enc.encode(&img, &mut out) {
		// For really small images, we might be able to save even more by
		// nuking the palette.
		if out.size < 4096 && LodePNGColorType::LCT_PALETTE.is_match(&out) {
			if let Some(out2) = enc.try_small(&img) {
				if out2.size < out.size && out2.size < src.len() {
					// We improved again!
					return Some(out2);
				}
			}
		}

		// We improved!
		if out.size < src.len() { return Some(out); }
	}

	None
}

#[track_caller]
#[allow(unsafe_code)]
/// # Unreachable Hint.
///
/// This is a simple unreachability wrapper that calls `unreachable!` when
/// debug assertions are enabled, or the quieter `hint::unreachable_unchecked`
/// when not.
///
/// Especially since the latter is unsafe, this helps prevent the compiler
/// from making stupid inlining decisions in hot blocks. Haha.
pub(crate) const fn unreachable() {
	#[cfg(debug_assertions)] unreachable!();
	#[cfg(not(debug_assertions))] unsafe { core::hint::unreachable_unchecked(); }
}



/// # Best Strategy.
///
/// This re-encodes the image (quickly) using each strategy, returning
/// whichever produced the smallest output.
///
/// Skipping zopfli here saves _a ton_ of processing time and (almost) never
/// changes the answer, so it's a shortcut worth taking.
fn best_strategy(
	img: &DecodedImage,
	enc: &mut LodePNGState,
	out: &mut EncodedPNG,
) -> LodePNGFilterStrategy {
	let mut best_size = usize::MAX;
	let mut best_strategy = LodePNGFilterStrategy::LFS_ZERO;

	for strategy in [
		LodePNGFilterStrategy::LFS_ZERO,
		LodePNGFilterStrategy::LFS_ONE,
		LodePNGFilterStrategy::LFS_TWO,
		LodePNGFilterStrategy::LFS_THREE,
		LodePNGFilterStrategy::LFS_FOUR,
		LodePNGFilterStrategy::LFS_MINSUM,
		LodePNGFilterStrategy::LFS_ENTROPY,
		LodePNGFilterStrategy::LFS_BRUTE_FORCE,
	] {
		enc.set_strategy(strategy);
		if enc.encode(img, out) && out.size < best_size {
			best_size = out.size;
			best_strategy = strategy;
		}
	}

	best_strategy
}
