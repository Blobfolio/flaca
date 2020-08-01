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
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

use flaca_core::image;
use fyi_witcher::{
	self,
	Witcher
};
use std::{
	ffi::OsStr,
	io::{
		self,
		Write,
	},
	path::PathBuf,
};



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
fn main() {
	let mut args = fyi_menu::parse_env_args(fyi_menu::FLAG_ALL);
	let mut progress: bool = false;
	let mut list: Option<String> = None;

	// Run through the arguments to see what we've got going on!
	let mut idx: usize = 0;
	let mut len: usize = args.len();
	while idx < len {
		match args[idx].as_str() {
			"-h" | "--help" => { return _help(); },
			"-V" | "--version" => { return _version(); },
			"-p" | "--progress" => { progress = true; },
			"-l" | "--list" =>
				if idx + 1 == len {
					fyi_menu::die(b"Missing file list.");
				}
				else {
					list.replace(args.remove(idx + 1));
					len -= 1;
				},
			_ => { break; }
		}

		idx += 1;
	}

	// Clear what we've checked.
	if idx > 0 {
		args.drain(0..idx);
	}

	// What path are we dealing with?
	let walk = match list {
		Some(p) => Witcher::read_paths_from_file(p),
		None => Witcher::from(args),
	}
		.filter(witch_filter)
		.collect::<Vec<PathBuf>>();

	if walk.is_empty() {
		fyi_menu::die(b"No images were found.");
	}
	// With progress.
	else if progress {
		fyi_witcher::progress_crunch(&walk, "Flaca", image::compress);
	}
	// Without progress.
	else {
		fyi_witcher::process(&walk, image::compress);
	}
}

#[allow(trivial_casts)] // Trivial though it may be, the code doesn't work without it!
/// Accept or Deny Files.
fn witch_filter(path: &PathBuf) -> bool {
	let bytes: &[u8] = unsafe { &*(path.as_os_str() as *const OsStr as *const [u8]) };
	let len: usize = bytes.len();

	len > 5 &&
	(
		bytes[len-4..len].eq_ignore_ascii_case(b".jpg") ||
		bytes[len-4..len].eq_ignore_ascii_case(b".png") ||
		bytes[len-5..len].eq_ignore_ascii_case(b".jpeg")
	)
}

#[cfg(not(feature = "man"))]
#[cold]
/// Print Help.
fn _help() {
	io::stdout().write_fmt(format_args!(
		r"
             ,--._,--.
           ,'  ,'   ,-`.
(`-.__    /  ,'   /
 `.   `--'        \__,--'-.
   `--/       ,-.  ______/
     (o-.     ,o- /
      `. ;        \    {}{}{}
       |:          \   Brute-force, lossless
      ,'`       ,   \  JPEG and PNG compression.
     (o o ,  --'     :
      \--','.        ;
       `;;  :       /
        ;'  ;  ,' ,'
        ,','  :  '
        \ \   :
         `

{}",
			"\x1b[38;5;199mFlaca\x1b[0;38;5;69m v",
			env!("CARGO_PKG_VERSION"),
			"\x1b[0m",
			include_str!("../../skel/help.txt")
	)).unwrap();
}

#[cfg(feature = "man")]
#[cold]
/// Print Help.
///
/// This is a stripped-down version of the help screen made specifically for
/// `help2man`, which gets run during the Debian package release build task.
fn _help() {
	io::stdout().write_all(&[
		b"Flaca ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n",
		env!("CARGO_PKG_DESCRIPTION").as_bytes(),
		b"\n\n",
		include_bytes!("../../skel/help.txt"),
		b"\n",
	].concat()).unwrap();
}

#[cold]
/// Print version and exit.
fn _version() {
	io::stdout().write_all(&[
		b"Flaca ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n"
	].concat()).unwrap();
}
