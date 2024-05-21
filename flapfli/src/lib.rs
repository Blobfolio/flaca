/*!
# Flaca
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

mod ffi;
mod lodepng;
mod zopflipng;

pub use ffi::EncodedImage;
pub use zopflipng::optimize;

use std::sync::atomic::AtomicI32;
use zopflipng::{
	deflate_part,
	SplitPoints,
	ZopfliState,
	ZOPFLI_MASTER_BLOCK_SIZE,
};



/// # Number of Zopfli Iterations.
pub static ZOPFLI_ITERATIONS: AtomicI32 = AtomicI32::new(0);
