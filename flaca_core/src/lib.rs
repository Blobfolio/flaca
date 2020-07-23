/*!
# Flaca

Brute-force, lossless JPEG and PNG compression.
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::pedantic)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

pub mod image;
pub mod encoder;
pub mod jpegtran;

use crate::image::ImageKind;
use encoder::{
	Encoder,
	Mozjpeg,
	Oxipng,
	Pngout,
	Zopflipng,
};
use fyi_witcher::utility::is_executable;
use std::{
	env,
	fs,
	path::PathBuf,
};

lazy_static::lazy_static! {
	static ref PNGOUT: PathBuf = Pngout::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
	static ref ZOPFLIPNG: PathBuf = Zopflipng::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));

	static ref PNGOUT_EXISTS: bool = PNGOUT.is_file();
	static ref ZOPFLIPNG_EXISTS: bool = ZOPFLIPNG.is_file();
}



#[must_use]
/// Bytes saved.
pub fn bytes_saved(before: u64, after: u64) -> u64 {
	if 0 == after || before <= after { 0 }
	else { before - after }
}

/// Find Executable.
pub fn find_executable<S> (name: S) -> Option<PathBuf>
where S: AsRef<str> {
	lazy_static::lazy_static! {
		// We only need to build a list of executable base paths once.
		static ref EXECUTABLE_DIRS: Vec<PathBuf> =
			env::var("PATH").unwrap_or_else(|_| "".to_string())
				.split(':')
				.filter_map(|x| fs::canonicalize(&x).ok()
					.and_then(|x|
						if x.is_dir() { Some(x) }
						else { None }
					)
				)
				.collect();
	}

	if ! EXECUTABLE_DIRS.is_empty() {
		for dir in EXECUTABLE_DIRS.as_slice() {
			let mut path = dir.clone();
			path.push(name.as_ref());
			if is_executable(&path) {
				return Some(path);
			}
		}
	}

	None
}

#[allow(unused_must_use)]
/// Encode.
pub fn encode_image(path: &PathBuf) {
	match ImageKind::from(path) {
		ImageKind::Jpeg => {
			Mozjpeg::encode(path);
		},
		ImageKind::Png => {
			Pngout::encode(path);
			Oxipng::encode(path);
			Zopflipng::encode(path);
		},
		ImageKind::None => {},
	}
}
