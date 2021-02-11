/*!
# Flaca

Flaca is a CLI tool for x86-64 Linux machines that simplifies the task of **losslessly** compressing JPEG and PNG images for use in production **web environments**.

It prioritizes compression over speed or resource modesty, and runs best on systems with multiple CPUs. There are only so many ways to be a JPEG, but calculating the optimal construction for a PNG takes a lot of work!

Compression is mainly achieved through the removal of metadata and optimization of pixel tables. Under the hood, Flaca leverages the `jpegtran` functionality from [`MozJPEG`](https://github.com/mozilla/mozjpeg) for JPEG images, and a combination of [Oxipng](https://github.com/shssoichiro/oxipng) and [Zopflipng](https://github.com/google/zopfli) for PNG images.



## Metadata

For web images, metadata is just so much wasted bandwidth. Stock photos in particular can be bloated 50% or more with embedded keywords and descriptions that browsers make zero use of. Removing that data — particularly at scale — leads to both lower hosting costs for site operators and faster page loads for visitors.

And it helps close the [digital divide](https://en.wikipedia.org/wiki/Digital_divide).

**However**, the removal of metadata is only "lossless" in the context of images destined for view in web browsers. Image editors, printers, and gallery programs extensively use metadata for all sorts of different reasons ranging from gamma correction to geolocation.

**Do not** run Flaca against your personal media library or raw design/print sources or else Google Photos won't know what to make of all your selfies!

If your personal computer is _that_ strapped for disk space, just buy an external hard drive. :)



## Installation

Installable `.deb` packages are included with each [release](https://github.com/Blobfolio/flaca/releases/latest). They should always work for the latest stable Debian and Ubuntu.



## Usage

It's easy. Just run `flaca [FLAGS] [OPTIONS] <PATH(S)>…`.

The following flags and options are available:
```bash
-h, --help           Prints help information
-l, --list <list>    Read file paths from this list (one per line).
-p, --progress       Show progress bar while minifying.
-V, --version        Prints version information
```

You can feed it any number of file or directory paths in one go, and/or toss it a text file using the `-l` option. Directories are recursively searched.

Flaca can cross filesystem and user boundaries, provided the user running the program has the relevant read/write access. (Not that you should run it as `root`, but if you did, images would still be owned by `www-data` or whatever after compression.)

Some quick examples:
```bash
# Compress one file.
flaca /path/to/image.jpg

# Tackle a whole folder at once with a nice progress bar:
flaca -p /path/to/assets

# Or load it up with a lot of places separately:
flaca /path/to/assets /path/to/favicon.png …
```
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
#![allow(clippy::map_err_ignore)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]



use flaca_core::image;
use fyi_menu::{
	Argue,
	ArgueError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use fyi_msg::Msg;
use fyi_witcher::{
	utility,
	Witcher,
	WITCHING_DIFF,
	WITCHING_QUIET,
	WITCHING_SUMMARIZE,
};
use std::{
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::PathBuf,
};



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
/// Main.
fn main() {
	match _main() {
		Err(ArgueError::WantsVersion) => {
			fyi_msg::plain!(concat!("Flaca v", env!("CARGO_PKG_VERSION")));
		},
		Err(ArgueError::WantsHelp) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
		Ok(_) => {},
	}
}

#[inline]
/// Actual Main.
fn _main() -> Result<(), ArgueError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	let flags: u8 =
		if args.switch2(b"-p", b"--progress") { WITCHING_SUMMARIZE | WITCHING_DIFF }
		else { WITCHING_QUIET | WITCHING_SUMMARIZE | WITCHING_DIFF };

	// Put it all together!
	Witcher::default()
		.with_filter(|p: &PathBuf| {
			let p: &[u8] = utility::path_as_bytes(p);
			let p_len: usize = p.len();

			// Check for either of three different extensions at once, while
			// keeping branching to a minimum. It looks a bit weird, but isn't
			// too complicated. :)
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
		.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x.as_ref())))
		.into_witching()
		.with_flags(flags)
		.with_labels("image", "images")
		.with_title(Msg::custom("Flaca", 199, "Reticulating splines\u{2026}"))
		.run(image::compress);

	Ok(())
}

#[cold]
/// # Print Help.
fn helper() {
	fyi_msg::plain!(concat!(
		r"
             ,--._,--.
           ,'  ,'   ,-`.
(`-.__    /  ,'   /
 `.   `--'        \__,--'-.
   `--/       ,-.  ______/
     (o-.     ,o- /
      `. ;        \    ", "\x1b[38;5;199mFlaca\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r"
       |:          \   Brute-force, lossless
      ,'`       ,   \  JPEG and PNG compression.
     (o o ,  --'     :
      \--','.        ;
       `;;  :       /
        ;'  ;  ,' ,'
        ,','  :  '
        \ \   :
         `

USAGE:
    flaca [FLAGS] [OPTIONS] <PATH(S)>...

FLAGS:
    -h, --help        Prints help information
    -p, --progress    Show progress bar while minifying.
    -V, --version     Prints version information

OPTIONS:
    -l, --list <list>    Read file paths from this list.

ARGS:
    <PATH(S)>...    One or more files or directories to compress.

OPTIMIZERS USED:
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Zopflipng <https://github.com/google/zopfli>
"
	));
}
