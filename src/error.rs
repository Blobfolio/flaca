/*!
# Flaca: Error
*/

use std::{
	error::Error,
	fmt,
};



#[derive(Debug, Copy, Clone)]
/// # Error type.
pub enum FlacaError {
	/// # File is empty.
	EmptyFile,
	/// # File is not a JPEG or PNG.
	InvalidImageType,
	/// # Parse issue.
	ParseFail,
	/// # Unable to read image.
	ReadFail,
	/// # Unable to save image.
	WriteFail,
}

impl Error for FlacaError {}

impl fmt::Display for FlacaError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(self.as_str())
	}
}

impl FlacaError {
	#[must_use]
	/// # As Str.
	pub const fn as_str(self) -> &'static str {
		match self {
			Self::EmptyFile => "The image is empty.",
			Self::InvalidImageType => "The image is not a JPEG or PNG.",
			Self::ParseFail => "The image is malformed.",
			Self::ReadFail => "Unable to read image.",
			Self::WriteFail => "Unable to save image.",
		}
	}
}
