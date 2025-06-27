/*!
# Flapfli.

This library contains a (mostly) Rust port of [`zopflipng`](https://github.com/google/zopfli/),
heavily optimized flaca's specific use cases (hence "fla" + "pfli").
*/

#![deny(
	clippy::allow_attributes_without_reason,
	clippy::correctness,
	unreachable_pub,
	unsafe_code,
)]

#![warn(
	clippy::complexity,
	clippy::nursery,
	clippy::pedantic,
	clippy::perf,
	clippy::style,

	clippy::allow_attributes,
	clippy::clone_on_ref_ptr,
	clippy::create_dir,
	clippy::filetype_is_file,
	clippy::format_push_string,
	clippy::get_unwrap,
	clippy::impl_trait_in_params,
	clippy::lossy_float_literal,
	clippy::missing_assert_message,
	clippy::missing_docs_in_private_items,
	clippy::needless_raw_strings,
	clippy::panic_in_result_fn,
	clippy::pub_without_shorthand,
	clippy::rest_pat_in_fully_bound_structs,
	clippy::semicolon_inside_block,
	clippy::str_to_string,
	clippy::string_to_string,
	clippy::todo,
	clippy::undocumented_unsafe_blocks,
	clippy::unneeded_field_pattern,
	clippy::unseparated_literal_suffix,
	clippy::unwrap_in_result,

	macro_use_extern_crate,
	missing_copy_implementations,
	missing_docs,
	non_ascii_idents,
	trivial_casts,
	trivial_numeric_casts,
	unused_crate_dependencies,
	unused_extern_crates,
	unused_import_braces,
)]

#![expect(clippy::redundant_pub_crate, reason = "Unresolvable.")]

mod deflate;
mod ffi;
mod lodepng;
mod zopflipng;

pub use deflate::set_zopfli_iterations;
use ffi::EncodedPNG;
use lodepng::{
	DecodedImage,
	LodePNGColorType,
	LodePNGFilterStrategy,
	LodePNGState,
};
use zopflipng::{
	deflate_part,
	ZOPFLI_MASTER_BLOCK_SIZE,
	ZopfliChunk,
	ZopfliState,
};



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
	let strategy = best_strategy(&img, &mut enc);

	// Now re-re-encode with zopfli and the best strategy.
	enc.set_strategy(strategy);
	enc.set_zopfli();
	let out = enc.encode(&img)?;

	// For really small images, we might be able to save even more by
	// nuking the palette.
	if
		out.size < 4096 &&
		LodePNGColorType::LCT_PALETTE.is_match(&out) &&
		let Some(out2) = enc.try_small(&img) &&
		out2.size < out.size &&
		out2.size < src.len()
	{
		Some(out2)
	}
	// We improved!
	else if out.size < src.len() { Some(out) }
	else { None }
}



/// # Best Strategy.
///
/// This re-encodes the image (quickly) using each strategy, returning
/// whichever produced the smallest output.
///
/// Skipping zopfli here saves _a ton_ of processing time and (almost) never
/// changes the answer, so it's a shortcut worth taking.
fn best_strategy(img: &DecodedImage, enc: &mut LodePNGState) -> LodePNGFilterStrategy {
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
		let Some(out) = enc.encode(img) else { continue; };
		if out.size < best_size {
			best_size = out.size;
			best_strategy = strategy;
		}
	}

	best_strategy
}
