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

use flaca_core::encode_image;
use fyi_menu::ArgList;
use fyi_witcher::{
	Witcher,
	Result,
};
use std::{
	ffi::OsStr,
	fs,
	io::{
		self,
		Write,
	},
	path::PathBuf,
};



/// -h | --help
const FLAG_HELP: u8     = 0b0001;
/// -p | --progress
const FLAG_PROGRESS: u8 = 0b0010;
/// -V | --version
const FLAG_VERSION: u8  = 0b0100;



fn main() -> Result<()> {
	let mut args = ArgList::default();
	args.expect();

	let flags = _flags(&mut args);
	// Help or Version?
	if 0 != flags & FLAG_HELP {
		_help();
		return Ok(());
	}
	else if 0 != flags & FLAG_VERSION {
		_version();
		return Ok(());
	}

	// What path are we dealing with?
	let walk = match args.pluck_opt(|x| x == "-l" || x == "--list") {
		Some(p) => unsafe { Witcher::from_file_custom(p, witch_filter) },
		None => unsafe { Witcher::custom(&args.expect_args(), witch_filter) },
	};

	if walk.is_empty() {
		return Err("No image files were found.".to_string());
	}

	// Without progress.
	if 0 == flags & FLAG_PROGRESS {
		walk.process(encode_image);
	}
	// With progress.
	else {
		walk.progress_crunch("Flaca", encode_image);
	}

	Ok(())
}

#[allow(clippy::needless_pass_by_value)] // Would if it were the expected signature!
#[allow(trivial_casts)] // Trivial though it may be, the code doesn't work without it!
/// Accept or Deny Files.
fn witch_filter(res: Result<jwalk::DirEntry<((), ())>, jwalk::Error>) -> Option<PathBuf> {
	res.ok()
		.and_then(|p| if p.file_type().is_dir() { None } else { Some(p) })
		.and_then(|p| fs::canonicalize(p.path()).ok())
		.and_then(|p| {
			let bytes: &[u8] = unsafe { &*(p.as_os_str() as *const OsStr as *const [u8]) };
			let len: usize = bytes.len();
			if
				len > 5 &&
				(
					bytes[len-5..len].eq_ignore_ascii_case(b".jpeg") ||
					bytes[len-4..len].eq_ignore_ascii_case(b".jpg") ||
					bytes[len-4..len].eq_ignore_ascii_case(b".png")
				)
			{ Some(p) }
			else { None }
		})
}

/// Fetch Flags.
fn _flags(args: &mut ArgList) -> u8 {
	let len: usize = args.len();
	if 0 == len { 0 }
	else {
		let mut flags: u8 = 0;
		let mut del = 0;
		let raw = args.as_mut_vec();

		// This is basically what `Vec.retain()` does, except we're hitting
		// multiple patterns at once and sending back the results.
		let ptr = raw.as_mut_ptr();
		unsafe {
			let mut idx: usize = 0;
			while idx < len {
				match (*ptr.add(idx)).as_str() {
					"-h" | "--help" => {
						flags |= FLAG_HELP;
						del += 1;
					},
					"-p" | "--progress" => {
						flags |= FLAG_PROGRESS;
						del += 1;
					},
					"-V" | "--version" => {
						flags |= FLAG_VERSION;
						del += 1;
					},
					_ => if del > 0 {
						ptr.add(idx).swap(ptr.add(idx - del));
					}
				}

				idx += 1;
			}
		}

		// Did we find anything? If so, run `truncate()` to free the memory
		// and return the flags.
		if del > 0 {
			raw.truncate(len - del);
			flags
		}
		else { 0 }
	}
}

#[cfg(not(feature = "man"))]
#[cold]
/// Print Help.
fn _help() {
	io::stdout().write_all({
		let mut s = String::with_capacity(2048);
		s.push_str(&format!(
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

",
			"\x1b[38;5;199mFlaca\x1b[0;38;5;69m v",
			env!("CARGO_PKG_VERSION"),
			"\x1b[0m"
		));
		s.push_str(include_str!("../misc/help.txt"));
		s.push('\n');
		s
	}.as_bytes()).unwrap();
}

#[cfg(feature = "man")]
#[cold]
/// Print Help.
///
/// This is a stripped-down version of the help screen made specifically for
/// `help2man`, which gets run during the Debian package release build task.
fn _help() {
	io::stdout().write_all({
		let mut s = String::with_capacity(1024);
		s.push_str("Flaca ");
		s.push_str(env!("CARGO_PKG_VERSION"));
		s.push('\n');
		s.push_str(env!("CARGO_PKG_DESCRIPTION"));
		s.push('\n');
		s.push('\n');
		s.push_str(include_str!("../misc/help.txt"));
		s.push('\n');
		s
	}.as_bytes()).unwrap();
}

#[cold]
/// Print version and exit.
fn _version() {
	io::stdout().write_all({
		let mut s = String::from("Flaca ");
		s.push_str(env!("CARGO_PKG_VERSION"));
		s.push('\n');
		s
	}.as_bytes()).unwrap();
}
