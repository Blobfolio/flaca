/*!
# Flaca: Errors
*/

use argyle::ArgyleError;
use fyi_msg::ProglessError;
use std::{
	error::Error,
	fmt,
};



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
	/// # Argyle Passthrough.
	Argue(ArgyleError),
	/// # Killed Early.
	Killed,
	/// # No Images.
	NoImages,
	/// # Max Resolution.
	MaxResolution,
	/// # Progress Passthrough.
	Progress(ProglessError),
	/// # Invalid Zopfli Iterations.
	ZopfliIterations,
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

impl From<ArgyleError> for FlacaError {
	#[inline]
	fn from(err: ArgyleError) -> Self { Self::Argue(err) }
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
			Self::Argue(e) => e.as_str(),
			Self::Killed => "The process was aborted early.",
			Self::NoImages => "No images were found.",
			Self::MaxResolution => "Pixel limits must be between 1..=4_294_967_295.",
			Self::Progress(e) => e.as_str(),
			Self::ZopfliIterations => "The number of (zopfli) lz77 iterations must be between 1..=2_147_483_647.",
		}
	}
}
