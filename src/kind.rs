/*!
# Flaca: Image Kind
*/

use crate::FlacaError;
use std::convert::TryFrom;



#[derive(Debug, Clone, Copy)]
/// # Image Kind.
///
/// This evaluates the file type from its headers, ensuring we process images
/// correctly even if they have the wrong extension (or don't process them if
/// they're bunk).
pub enum ImageKind {
	/// Jpeg.
	Jpeg,
	/// Png.
	Png,
}

impl TryFrom<&[u8]> for ImageKind {
	type Error = FlacaError;

	fn try_from(src: &[u8]) -> Result<Self, Self::Error> {
		// `imghdr` will panic if the slice is too small to contain headers.
		if src.len() < 8 {
			return Err(FlacaError::InvalidImageType);
		}

		match imghdr::from_bytes(src) {
			Some(imghdr::Type::Png) => Ok(Self::Png),
			Some(imghdr::Type::Jpeg) => Ok(Self::Jpeg),
			_ => Err(FlacaError::InvalidImageType),
		}
	}
}
