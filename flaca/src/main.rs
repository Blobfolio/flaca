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



use argyle::{
	Argue,
	ArgyleError,
	FLAG_HELP,
	FLAG_REQUIRED,
	FLAG_VERSION,
};
use flaca_core::image;
use fyi_msg::{
	Msg,
	Progless,
};
use dactyl::{
	NicePercent,
	NiceU64,
};
use dowser::{
	Dowser,
	utility::du,
};
use num_traits::cast::FromPrimitive;
use rayon::iter::{
	IntoParallelRefIterator,
	ParallelIterator,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	os::unix::ffi::OsStrExt,
	path::{
		Path,
		PathBuf,
	},
};



#[allow(clippy::if_not_else)] // Code is confusing otherwise.
/// Main.
fn main() {
	match _main() {
		Ok(_) => {},
		Err(ArgyleError::WantsVersion) => {
			println!(concat!("Flaca v", env!("CARGO_PKG_VERSION")));
		},
		Err(ArgyleError::WantsHelp) => {
			helper();
		},
		Err(e) => {
			Msg::error(e).die(1);
		},
	}
}

#[inline]
/// Actual Main.
fn _main() -> Result<(), ArgyleError> {
	// Parse CLI arguments.
	let args = Argue::new(FLAG_HELP | FLAG_REQUIRED | FLAG_VERSION)?
		.with_list();

	// Put it all together!
	let paths = Vec::<PathBuf>::try_from(
		Dowser::default()
			.with_filter(|p: &Path| p.extension()
				.map_or(
					false,
					|e| {
						let ext = e.as_bytes().to_ascii_lowercase();
						ext == b"jpg" || ext == b"png" || ext == b"jpeg"
					}
				)
			)
			.with_paths(args.args().iter().map(|x| OsStr::from_bytes(x.as_ref())))
	).map_err(|_| ArgyleError::Custom("No images were found."))?;

	// Sexy run-through.
	if args.switch2(b"-p", b"--progress") {
		// Check file sizes before we start.
		let before: u64 = du(&paths);

		// Boot up a progress bar.
		let progress = Progless::steady(u32::try_from(paths.len()).map_err(|_| ArgyleError::Custom("Only 4,294,967,295 files can be crunched at one time."))?)
			.with_title(Some(Msg::custom("Flaca", 199, "Reticulating splines\u{2026}")));

		// Process!
		paths.par_iter().for_each(|x| {
			let tmp = x.to_string_lossy();
			progress.add(&tmp);
			image::compress(x);
			progress.remove(&tmp);
		});

		// Finish up.
		let elapsed = progress.finish();
		let after: u64 = du(&paths);

		// Build and print a summary.
		if after > 0 && after < before {
			// Show a percentage difference if we can.
			if let (Some(p1), Some(p2)) = (f64::from_u64(before - after), f64::from_u64(before)) {
				Msg::success(format!(
					"Crunched {} image{} in {}, saving {} bytes \x1b[2m({})\x1b[0m.",
					NiceU64::from(paths.len()),
					if paths.len() == 1 { "" } else { "s" },
					elapsed.as_str(),
					NiceU64::from(before - after).as_str(),
					NicePercent::from(p1 / p2).as_str()
				)).print();
			}
			// Otherwise just the bytes.
			else {
				Msg::success(format!(
					"Crunched {} image{} in {}, saving {} bytes.",
					NiceU64::from(paths.len()),
					if paths.len() == 1 { "" } else { "s" },
					elapsed.as_str(),
					NiceU64::from(before - after).as_str()
				)).print();
			}
		}
		else {
			Msg::done(format!(
					"Crunched {} image{} in {}, but no savings were possible.",
					NiceU64::from(paths.len()),
					if paths.len() == 1 { "" } else { "s" },
					elapsed.as_str()
				)).print();
		}
	}
	else {
		paths.par_iter().for_each(|x| {
			image::compress(x);
		});
	}

	Ok(())
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
