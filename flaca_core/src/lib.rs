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

use encoder::{
	Encoder,
	Jpegoptim,
	Mozjpeg,
	Oxipng,
	Pngout,
	Zopflipng,
};
use fyi_msg::{
	Flags,
	Msg,
	traits::Printable,
};
use fyi_witcher::utility::is_executable;
use std::{
	borrow::Borrow,
	env,
	fs,
	path::PathBuf,
};

lazy_static::lazy_static! {
	static ref JPEGOPTIM: PathBuf = Jpegoptim::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
	static ref MOZJPEG: PathBuf = Mozjpeg::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
	static ref OXIPNG: PathBuf = Oxipng::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
	static ref PNGOUT: PathBuf = Pngout::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
	static ref ZOPFLIPNG: PathBuf = Zopflipng::find().unwrap_or_else(|_| PathBuf::from("/dev/null"));
}



/// Bytes saved.
pub fn bytes_saved(before: u64, after: u64) -> u64 {
	if 0 < after && after < before {
		before - after
	}
	else { 0 }
}

/// Dependency check.
pub fn check_dependencies() {
	if ! JPEGOPTIM.is_file() {
		die(format!("Missing: {} <{}>", Jpegoptim::NAME, Jpegoptim::URL));
	}
	if ! MOZJPEG.is_file() {
		die(format!("Missing: {} <{}>", Mozjpeg::NAME, Mozjpeg::URL));
	}
	if ! OXIPNG.is_file() {
		die(format!("Missing: {} <{}>", Oxipng::NAME, Oxipng::URL));
	}
	if ! PNGOUT.is_file() {
		die(format!("Missing: {} <{}>", Pngout::NAME, Pngout::URL));
	}
	if ! ZOPFLIPNG.is_file() {
		die(format!("Missing: {} <{}>", Zopflipng::NAME, Zopflipng::URL));
	}
}

/// Error and Exit.
pub fn die<S> (msg: S)
where S: Borrow<str> {
	Msg::error(msg)
		.print(0, Flags::TO_STDERR);

	std::process::exit(1);
}

/// Find Executable.
pub fn find_executable<S> (name: S) -> Option<PathBuf>
where S: Into<String> {
	lazy_static::lazy_static! {
		// We only need to build a list of executable base paths once.
		static ref EXECUTABLE_DIRS: Vec<PathBuf> =
			env::var("PATH").unwrap_or_else(|_| "".to_string())
				.split(':')
				.filter_map(|x| {
					if let Ok(path) = fs::canonicalize(&x) {
						if path.is_dir() {
							Some(path)
						}
						else { None }
					}
					else { None }
				})
				.collect();
	}

	if ! EXECUTABLE_DIRS.is_empty() {
		let name = name.into();
		for dir in EXECUTABLE_DIRS.as_slice() {
			let mut path = dir.clone();
			path.push(&name);
			if is_executable(&path) {
				return Some(path);
			}
		}
	}

	None
}
