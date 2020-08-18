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
use fyi_menu::Argue;
use fyi_msg::MsgKind;
use fyi_witcher::{
	Witcher,
	WITCHING_DIFF,
	WITCHING_QUIET,
	WITCHING_SUMMARIZE,
};



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
fn main() {
	// Parse CLI arguments.
	let args = Argue::new()
		.with_any()
		.with_version(versioner)
		.with_help(helper)
		.with_list();

	let mut flags: u8 = WITCHING_QUIET | WITCHING_SUMMARIZE | WITCHING_DIFF;
	if args.switch2("-p", "--progress") {
		flags &= ! WITCHING_QUIET;
	}

	// Put it all together!
	Witcher::default()
		.with_ext3(b".jpg", b".png", b".jpeg")
		.with_paths(args.args())
		.into_witching()
		.with_flags(flags)
		.with_labels("image", "images")
		.with_title(MsgKind::new("Flaca", 199).into_msg("Reticulating splines\u{2026}"))
		.run(image::compress);
}

#[cfg(not(feature = "man"))]
#[cold]
/// Print Help.
fn helper(_: Option<&str>) {
	use std::io::Write;

	std::io::stdout().write_fmt(format_args!(
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
fn helper(_: Option<&str>) {
	use std::io::Write;

	let writer = std::io::stdout();
	let mut handle = writer.lock();

	handle.write_all(b"Flaca ").unwrap();
	handle.write_all(env!("CARGO_PKG_VERSION").as_bytes()).unwrap();
	handle.write_all(b"\n").unwrap();
	handle.write_all(env!("CARGO_PKG_DESCRIPTION").as_bytes()).unwrap();
	handle.write_all(b"\n\n").unwrap();
	handle.write_all(include_bytes!("../../skel/help.txt")).unwrap();
	handle.write_all(b"\n").unwrap();

	handle.flush().unwrap();
}

/// Print Version.
fn versioner() {
	use std::io::Write;
	let writer = std::io::stdout();
	let mut handle = writer.lock();

	handle.write_all(b"Flaca ").unwrap();
	handle.write_all(env!("CARGO_PKG_VERSION").as_bytes()).unwrap();
	handle.write_all(b"\n").unwrap();

	handle.flush().unwrap();
}
