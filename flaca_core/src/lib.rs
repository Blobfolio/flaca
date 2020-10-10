/*!
# Flaca

Brute-force, lossless JPEG and PNG compression.
*/

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]
#![warn(macro_use_extern_crate)]
#![warn(missing_copy_implementations)]
#![warn(missing_debug_implementations)]
#![warn(missing_docs)]
#![warn(non_ascii_idents)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unreachable_pub)]
#![warn(unused_crate_dependencies)]
#![warn(unused_extern_crates)]
#![warn(unused_import_braces)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



pub mod image;
pub mod jpegtran;

use fyi_witcher::utility::is_executable;
use std::{
	env,
	fs,
	path::PathBuf,
};



/// Generic result type.
pub type Result<T, E = ()> = std::result::Result<T, E>;



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
