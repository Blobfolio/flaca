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

#![allow(clippy::module_name_repetitions)]



mod error;
mod image;

pub(crate) use error::FlacaError;
pub(crate) use image::FlacaImage;

use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use dowser::{
	Dowser,
	Extension,
};
use fyi_msg::{
	BeforeAfter,
	Msg,
	MsgKind,
	Progless,
};
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::PathBuf,
	sync::{
		Arc,
		atomic::{
			AtomicBool,
			AtomicU64,
			Ordering::SeqCst,
		},
	},
};



/// # Main.
///
/// This shell provides us a way to easily handle error responses. Actual
/// processing is done by `_main()`.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(FlacaError::Argue(ArgyleError::WantsVersion)) => {
			println!(concat!("Flaca v", env!("CARGO_PKG_VERSION")));
		},
		Err(FlacaError::Argue(ArgyleError::WantsHelp)) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
	}
}

#[allow(clippy::option_if_let_else)] // This looks bad.
#[inline]
/// # Actual Main.
///
/// This is the actual main, allowing us to easily bubble errors.
fn _main() -> Result<(), FlacaError> {
	const E_JPG: Extension = Extension::new3(*b"jpg");
	const E_PNG: Extension = Extension::new3(*b"png");
	const E_JPEG: Extension = Extension::new4(*b"jpeg");

	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Figure out which kinds we're doing.
	let jpeg: bool = ! args.switch2(b"--no-jpeg", b"--no-jpg");
	let png: bool = ! args.switch(b"--no-png");
	let progress = args.switch2(b"-p", b"--progress");

	let iter = Dowser::default()
		.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x)));

	// Find files!
	let paths: Vec<PathBuf> = match (jpeg, png) {
		// Both.
		(true, true) => iter.into_vec(|p|
			Extension::try_from3(p).map_or_else(
				|| Some(E_JPEG) == Extension::try_from4(p),
				|e| e == E_JPG || e == E_PNG
			)
		),
		// JPEG.
		(true, false) => iter.into_vec(|p|
			Extension::try_from3(p).map_or_else(
				|| Some(E_JPEG) == Extension::try_from4(p),
				|e| e == E_JPG
			)
		),
		// PNG.
		(false, true) => iter.into_vec(|p| Some(E_PNG) == Extension::try_from3(p)),
		// Nothing?!
		(false, false) => Vec::new(),
	};

	if paths.is_empty() {
		return Err(FlacaError::NoImages);
	}

	// Controls for early termination.
	let killed = Arc::from(AtomicBool::new(false));
	let k2 = Arc::clone(&killed);

	// Sexy run-through.
	if progress {
		// Boot up a progress bar.
		let progress = Progless::try_from(paths.len())
			.map_err(|_| FlacaError::ProgressOverflow)?
			.with_title(Some(Msg::custom("Flaca", 199, "Reticulating splines\u{2026}")));

		// Keep track of the before and after file sizes as we go.
		let before: AtomicU64 = AtomicU64::new(0);
		let after: AtomicU64 = AtomicU64::new(0);

		// Intercept CTRL+C so we can gracefully shut down.
		let p2 = progress.clone();
		let _res = ctrlc::set_handler(move || {
			k2.store(true, SeqCst);
			p2.set_title(Some(Msg::warning("Early shutdown in progress.")));
		});

		// Process!
		paths.par_iter().for_each(|x|
			if ! killed.load(SeqCst) {
				// Encode if we can.
				if let Some(mut enc) = FlacaImage::new(x, jpeg, png) {
					let tmp = x.to_string_lossy();
					progress.add(&tmp);

					let (b, a) = enc.compress();
					before.fetch_add(b, SeqCst);
					after.fetch_add(a, SeqCst);

					progress.remove(&tmp);
				}
				// Bump the count if we can't.
				else {
					progress.increment();
				}
			}
		);

		// Finish up.
		progress.finish();

		if ! killed.load(SeqCst) {
			// Print a summary.
			progress.summary(MsgKind::Crunched, "image", "images")
				.with_bytes_saved(BeforeAfter::from((
					before.load(SeqCst),
					after.load(SeqCst),
				)))
				.print();
		}
	}
	else {
		// Intercept CTRL+C so we can gracefully shut down.
		let _res = ctrlc::set_handler(move || { k2.store(true, SeqCst); });

		// Process!
		paths.par_iter().for_each(|x|
			if ! killed.load(SeqCst) {
				if let Some(mut enc) = FlacaImage::new(x, jpeg, png) {
					let _res = enc.compress();
				}
			}
		);
	}

	// Early abort?
	if killed.load(SeqCst) { Err(FlacaError::Killed) }
	else { Ok(()) }
}

#[cold]
/// # Print Help.
fn helper() {
	println!(concat!(
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
    -h, --help        Print help information and exit.
        --no-jpeg     Skip JPEG images.
        --no-png      Skip PNG images.
    -p, --progress    Show progress bar while minifying.
    -V, --version     Print version information and exit.

OPTIONS:
    -l, --list <FILE> Read (absolute) image and/or directory paths from this
                      text file, one entry per line.

ARGS:
    <PATH(S)>...      One or more image and/or directory paths to losslessly
                      compress.

OPTIMIZERS USED:
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Zopflipng <https://github.com/google/zopfli>
"
	));
}
