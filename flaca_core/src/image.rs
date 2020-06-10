/*!
# Flaca: Image Business
*/

use std::fmt;



#[derive(Debug, Clone, Copy, Hash, PartialEq)]
/// Image Kind.
pub enum ImageKind {
	/// Jpeg.
	Jpeg,
	/// Png.
	Png,
	/// Neither.
	None,
}

impl fmt::Display for ImageKind {
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match *self {
			Self::Jpeg => "JPEG",
			Self::Png => "PNG",
			Self::None => "None",
		})
	}
}

impl ImageKind {
	#[must_use]
	/// Suffix.
	pub fn suffix(self) -> Option<String> {
		match self {
			Self::Jpeg => Some(".jpg".to_string()),
			Self::Png => Some(".png".to_string()),
			Self::None => None,
		}
	}
}
