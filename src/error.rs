/*!
# Flaca: Errors
*/

use argyle::ArgyleError;
use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error type.
pub(super) enum FlacaError {
	/// # Argyle passthrough.
	Argue(ArgyleError),
	/// # Killed Early.
	Killed,
	/// # No images.
	NoImages,
	/// # Progress Overflow.
	ProgressOverflow,
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

impl FlacaError {
	#[must_use]
	/// # As Str.
	pub(super) const fn as_str(self) -> &'static str {
		match self {
			Self::Argue(e) => e.as_str(),
			Self::Killed => "The process was aborted early.",
			Self::NoImages => "No images were found.",
			Self::ProgressOverflow => "Progress can only be displayed for up to 4,294,967,295 images. Try again with fewer images or without the -p/--progress flag.",
		}
	}
}
