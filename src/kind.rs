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
		// If the source is big enough for headers, keep going!
		if src.len() > 12 {
			// PNG has just one way to be!
			if src[..8] == [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A] {
				return Ok(Self::Png);
			}

			// JPEG has a lot of different possible headers.
			if
				src[..3] == [0xFF, 0xD8, 0xFF] &&
				(
					src[3] == 0xDB ||
					src[3] == 0xEE ||
					(src[3..12] == [0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46, 0x00, 0x01]) ||
					(src[3] == 0xE1 && src[6..12] == [0x45, 0x78, 0x69, 0x66, 0x00, 0x00])
				)
			{
				return Ok(Self::Jpeg);
			}
		}

		Err(FlacaError::InvalidImageType)
	}
}
