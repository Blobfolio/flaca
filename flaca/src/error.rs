/*!
# Flaca: Errors
*/

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
      `. ;        \    ", "\x1b[38;5;199mFlaca\x1b[0;38;5;69m v", env!("CARGO_PKG_VERSION"), "\x1b[0m", r#"
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
    -p, --progress    Show pretty progress while minifying.
    -V, --version     Print version information and exit.

OPTIONS:
    -j <NUM>          Limit parallelization to this many threads (instead of
                      giving each logical core its own image to work on). If
                      negative, the value will be subtracted from the total
                      number of logical cores.
    -l, --list <FILE> Read (absolute) image and/or directory paths from this
                      text file — or STDIN if "-" — one entry per line, instead
                      of or in addition to (actually trailing) <PATH(S)>.
        --max-resolution <NUM>
                      Skip images containing more than <NUM> total pixels to
                      avoid potential OOM errors during decompression.
                      [default: ~4.29 billion]
    -z <NUM>          Run NUM lz77 backward/forward iterations during zopfli
                      PNG encoding passes. More iterations yield better
                      compression (up to a point), but require *significantly*
                      longer processing times. In practice, values beyond 500
                      are unlikely to save more than a few bytes, and could
                      take *days* to complete! Haha. [default: 20 or 60,
                      depending on the file size]
ARGS:
    <PATH(S)>...      One or more image and/or directory paths to losslessly
                      compress.

EARLY EXIT:
    Press "#, "\x1b[38;5;208mCTRL\x1b[0m+\x1b[38;5;208mC\x1b[0m once to quit as soon as the already-in-progress operations
    have finished (ignoring any pending images still in the queue).

    Press \x1b[38;5;208mCTRL\x1b[0m+\x1b[38;5;208mC\x1b[0m a second time if you need to exit IMMEDIATELY, but note that
    doing so may leave artifacts (temporary files) behind, and in rare cases,
    lead to image corruption.

OPTIMIZERS USED:
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Zopflipng <https://github.com/google/zopfli>
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
			Self::Skipped => "",
			Self::Vanished => "vanished!",
			Self::Write => "write error",
		}
	}
}



#[derive(Debug, Copy, Clone)]
/// # General/Deal-Breaking Errors.
pub(super) enum FlacaError {
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

	/// # Print Help (Not an Error).
	PrintHelp,

	/// # Print Version (Not an Error).
	PrintVersion,
}

impl AsRef<str> for FlacaError {
	#[inline]
	fn as_ref(&self) -> &str { self.as_str() }
}

impl Error for FlacaError {}

impl fmt::Display for FlacaError {
	#[inline]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl From<ProglessError> for FlacaError {
	#[inline]
	fn from(err: ProglessError) -> Self { Self::Progress(err) }
}

impl FlacaError {
	#[must_use]
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::Killed => "The process was aborted early.",
			Self::ListFile => "Invalid -l/--list text file.",
			Self::NoImages => "No images were found.",
			Self::MaxResolution => "Pixel limits must be between 1..=4_294_967_295.",
			Self::Progress(e) => e.as_str(),
			Self::ZopfliIterations => "The number of (zopfli) lz77 iterations must be between 1..=2_147_483_647.",
			Self::PrintHelp => HELP,
			Self::PrintVersion => concat!("Flaca v", env!("CARGO_PKG_VERSION")),
		}
	}
}
