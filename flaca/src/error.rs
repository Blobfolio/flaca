/*!
# Flaca: Errors
*/

use fyi_ansi::{
	ansi,
	csi,
	dim,
};
use fyi_msg::ProglessError;
use std::{
	error::Error,
	fmt,
};



/// # Help Text.
const HELP: &str = concat!(r"
             ,--._,--.
           ,'  ,'   ,-`.
(`-.__    /  ,'   /
 `.   `--'        \__,--'-.
   `--/       ,-.  ______/
     (o-.     ,o- /
      `. ;        \    ", csi!(199), "Flaca", ansi!((cornflower_blue) " v", env!("CARGO_PKG_VERSION")), r#"
       |:          \   Brute-force, lossless GIF,
      ,'`       ,   \  JPEG, and PNG compression.
     (o o ,  --'     :
      \--','.        ;
       `;;  :       /
        ;'  ;  ,' ,'
        ,','  :  '
        \ \   :
         `
"#, ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
USAGE:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    flaca [FLAGS] [OPTIONS] <PATH(S)>...

", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
FLAGS:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    -h, --help             Print help information and exit.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --no-gif           Skip GIF images.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --no-jpeg          Skip JPEG images.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --no-png           Skip PNG images.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --no-symlinks      Ignore symlinks (rather than following them).", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --preserve-times   (Try to) preserve the original file access and
                           modification times when resaving an image.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
    -p, --progress         Show pretty progress while minifying.", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
    -V, --version          Print version information and exit.
", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
OPTIONS:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    -j <NUM>               Limit parallelization to this many threads (instead
                           of giving each logical core its own image to work
                           on). If negative, the value will be subtracted from
                           the total number of logical cores.", ansi!((dim, violet) "
                           ----------------------------------------------------"), r#"
    -l, --list <FILE>      Read (absolute) image and/or directory paths from
                           this text file — or STDIN if "-" — one entry per
                           line, instead of or in addition to any trailing
                           <PATH(S)>."#, ansi!((dim, violet) "
                           ----------------------------------------------------"), "
        --max-pixels <NUM> Skip images containing more than <NUM> total pixels
                           to avoid potential OOM errors during decompression.
                           [default: ~4.29 billion]", ansi!((dim, violet) "
                           ----------------------------------------------------"), "
    -z <NUM>               Run <NUM> lz77 backward/forward iterations during
                           zopfli PNG encoding passes. More iterations yield
                           better compression (up to a point), but require
                           *significantly* longer processing times.
                           [default: 20 or 60, depending on the file size]
", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
ARGS:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    <PATH(S)>...           One or more image and/or directory paths to
                           losslessly re-compress.
", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
EARLY EXIT:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    Press ", ansi!((dark_orange) "CTRL"), "+", ansi!((dark_orange) "C"), " once to quit as soon as the already-in-progress operations
    have finished (ignoring any pending images still in the queue).

    Press ", ansi!((dark_orange) "CTRL"), "+", ansi!((dark_orange) "C"), " a second time if you need to exit IMMEDIATELY, but note that
    doing so may leave artifacts (temporary files) behind, and in rare cases,
    lead to image corruption.
", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
OPTIMIZERS USED:", ansi!((dim, violet) "
-------------------------------------------------------------------------------"), "
    Gifsicle  <", ansi!((cyan) "https://github.com/kohler/gifsicle"), ">
    MozJPEG   <", ansi!((cyan) "https://github.com/mozilla/mozjpeg"), ">
    Oxipng    <", ansi!((cyan) "https://github.com/shssoichiro/oxipng"), ">
    Zopflipng <", ansi!((cyan) "https://github.com/google/zopfli"), ">
");



#[derive(Debug, Copy, Clone)]
/// # Encoding Errors.
pub(super) enum EncodingError {
	/// # Empty File.
	Empty,

	/// # Wrong/Unknown Format.
	Format,

	/// # Read Error.
	Read,

	/// # Resolution.
	Resolution,

	/// # Intentionally Skipped.
	Skipped,

	/// # TBD Gif.
	TbdGif,

	/// # Vanished.
	Vanished,

	/// # Write Error.
	Write,
}

impl EncodingError {
	#[must_use]
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::Empty => "empty file",
			Self::Format => "invalid format",
			Self::Read => "read error",
			Self::Resolution => "too big",
			Self::Skipped | Self::TbdGif => "",
			Self::Vanished => "vanished!",
			Self::Write => "write error",
		}
	}
}



#[derive(Debug, Clone)]
/// # General/Deal-Breaking Errors.
pub(super) enum FlacaError {
	/// # Invalid CLI Arg.
	InvalidCli(String),

	/// # Killed Early.
	Killed,

	/// # List File.
	ListFile,

	/// # No Images.
	NoImages,

	/// # Max Resolution.
	MaxResolution,

	/// # Progress Passthrough.
	Progress(ProglessError),

	/// # Invalid Zopfli Iterations.
	ZopfliIterations,

	/// # Duplicate Zopfli Iterations.
	ZopfliIterations2,

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl Error for FlacaError {}

impl fmt::Display for FlacaError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		let prefix = self.as_str();
		match self {
			Self::InvalidCli(s) => write!(
				f,
				concat!("{} ", dim!("{}")),
				prefix,
				s,
			),
			_ => f.write_str(prefix),
		}
	}
}

impl From<ProglessError> for FlacaError {
	#[inline]
	fn from(err: ProglessError) -> Self { Self::Progress(err) }
}

impl FlacaError {
	#[must_use]
	/// # As Str.
	pub(super) const fn as_str(&self) -> &'static str {
		match self {
			Self::InvalidCli(_) => "Invalid/unknown argument:",
			Self::Killed => "The process was killed. 🕱",
			Self::ListFile => "Invalid -l/--list text file.",
			Self::NoImages => "No images were found.",
			Self::MaxResolution => "Pixel limits must be between 1..=4_294_967_295.",
			Self::Progress(e) => e.as_str(),
			Self::ZopfliIterations => "The number of (zopfli) lz77 iterations must be between 1..=2_147_483_647.",
			Self::ZopfliIterations2 => "The -z option can only be set once.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("Flaca v", env!("CARGO_PKG_VERSION")),
		}
	}
}
