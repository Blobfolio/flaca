/*!
# Flaca: Error
*/

use argyle::ArgyleError;
use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error type.
pub enum FlacaError {
	/// # Argyle passthrough.
	Argue(ArgyleError),
	/// # File is empty.
	EmptyFile,
	/// # File is not a JPEG or PNG.
	InvalidImageType,
	/// # Killed Early.
	Killed,
	/// # No images.
	NoImages,
	/// # Parse issue.
	ParseFail,
	/// # Progress Overflow.
	ProgressOverflow,
	/// # Unable to read image.
	ReadFail,
	/// # Temporary Failure.
	TmpDir,
	/// # Unable to save image.
	WriteFail,
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
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::Argue(e) => e.as_str(),
			Self::EmptyFile => "The image is empty.",
			Self::InvalidImageType => "The image is not a JPEG or PNG.",
			Self::Killed => "The process was aborted early.",
			Self::NoImages => "No images were found.",
			Self::ParseFail => "The image is malformed.",
			Self::ProgressOverflow => "Progress can only be displayed for up to 4,294,967,295 images. Try again with fewer images or without the -p/--progress flag.",
			Self::ReadFail => "Unable to read image.",
			Self::TmpDir => "Unable to manage temporary storage.",
			Self::WriteFail => "Unable to save image.",
		}
	}
}
