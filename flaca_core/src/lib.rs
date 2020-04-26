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

#[macro_use]
extern crate lazy_static;

extern crate fyi_core;
extern crate imghdr;

pub mod image;
pub mod encoder;

use encoder::*;
use fyi_core::{
	Msg,
	Prefix,
	traits::{
		AbsPath,
		PathProps,
	},
};
use std::{
	env,
	path::PathBuf,
};

lazy_static! {
	static ref JPEGOPTIM: PathBuf = Jpegoptim::find().unwrap_or(PathBuf::from("/dev/null"));
	static ref MOZJPEG: PathBuf = Mozjpeg::find().unwrap_or(PathBuf::from("/dev/null"));
	static ref OXIPNG: PathBuf = Oxipng::find().unwrap_or(PathBuf::from("/dev/null"));
	static ref PNGOUT: PathBuf = Pngout::find().unwrap_or(PathBuf::from("/dev/null"));
	static ref ZOPFLIPNG: PathBuf = Zopflipng::find().unwrap_or(PathBuf::from("/dev/null"));
}



/// Dependency check.
pub fn check_dependencies() {
	if false == JPEGOPTIM.is_file() {
		die(format!("Missing: {} <{}>", Jpegoptim::NAME, Jpegoptim::URL));
	}
	if false == MOZJPEG.is_file() {
		die(format!("Missing: {} <{}>", Mozjpeg::NAME, Mozjpeg::URL));
	}
	if false == OXIPNG.is_file() {
		die(format!("Missing: {} <{}>", Oxipng::NAME, Oxipng::URL));
	}
	if false == PNGOUT.is_file() {
		die(format!("Missing: {} <{}>", Pngout::NAME, Pngout::URL));
	}
	if false == ZOPFLIPNG.is_file() {
		die(format!("Missing: {} <{}>", Zopflipng::NAME, Zopflipng::URL));
	}
}

/// Error and Exit.
pub fn die<S> (msg: S)
where S: Into<String> {
	Msg::new(msg.into())
		.with_prefix(Prefix::Error)
		.print();

	std::process::exit(1);
}

/// Find Executable.
pub fn find_executable<S> (name: S) -> Option<PathBuf>
where S: Into<String> {
	lazy_static! {
		// We only need to build a list of executable base paths once.
		static ref EXECUTABLE_DIRS: Vec<PathBuf> =
			format!("{}", env::var("PATH").unwrap_or("".to_string()))
				.split(":")
				.filter_map(|ref x| {
					let path = PathBuf::from(x);
					match path.is_dir() {
						true => Some(path.to_path_buf_abs()),
						false => None,
					}
				})
				.collect();
	}

	if false == EXECUTABLE_DIRS.is_empty() {
		let name = name.into();
		for dir in EXECUTABLE_DIRS.as_slice() {
			let mut path = dir.clone();
			path.push(&name);
			if path.is_executable() {
				return Some(path);
			}
		}
	}

	None
}
