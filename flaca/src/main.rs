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



use flaca_core::image;
use fyi_menu::{
	Argue,
	FLAG_REQUIRED,
};
use fyi_msg::{
	Msg,
	MsgKind,
};
use fyi_witcher::{
	utility,
	Witcher,
	WITCHING_DIFF,
	WITCHING_QUIET,
	WITCHING_SUMMARIZE,
};
use std::path::PathBuf;



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
fn main() {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_REQUIRED)
		.with_version(b"Flaca", env!("CARGO_PKG_VERSION").as_bytes())
		.with_help(helper)
		.with_list();

	let flags: u8 =
		if args.switch2("-p", "--progress") { WITCHING_SUMMARIZE | WITCHING_DIFF }
		else { WITCHING_QUIET | WITCHING_SUMMARIZE | WITCHING_DIFF };

	// Put it all together!
	Witcher::default()
		.with_filter(|p: &PathBuf| {
			let p: &[u8] = utility::path_as_bytes(p);
			let p_len: usize = p.len();

			p_len > 5 &&
			p[p_len - 1].to_ascii_lowercase() == b'g' &&
			(
				(
					p[p_len - 4] == b'.' &&
					(
						(
							p[p_len - 3].to_ascii_lowercase() == b'j' &&
							p[p_len - 2].to_ascii_lowercase() == b'p'
						) ||
						(
							p[p_len - 3].to_ascii_lowercase() == b'p' &&
							p[p_len - 2].to_ascii_lowercase() == b'n'
						)
					)
				) ||
				(
					p[p_len - 5] == b'.' &&
					p[p_len - 4].to_ascii_lowercase() == b'j' &&
					p[p_len - 3].to_ascii_lowercase() == b'p' &&
					p[p_len - 2].to_ascii_lowercase() == b'e'
				)
			)

		})
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
	Msg::from(format!(
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
	)).print();
}

#[cfg(feature = "man")]
#[cold]
/// Print Help.
///
/// This is a stripped-down version of the help screen made specifically for
/// `help2man`, which gets run during the Debian package release build task.
fn helper(_: Option<&str>) {
	Msg::from([
		b"Flaca ",
		env!("CARGO_PKG_VERSION").as_bytes(),
		b"\n",
		env!("CARGO_PKG_DESCRIPTION").as_bytes(),
		b"\n\n",
		include_bytes!("../../skel/help.txt"),
	].concat())
		.print();
}
