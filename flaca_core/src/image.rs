/*!
# Flaca: Image Business
*/

use std::{
	fmt,
	path::PathBuf,
};



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

impl Default for ImageKind {
	fn default() -> Self {
		Self::None
	}
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

impl From<&PathBuf> for ImageKind {
	/// From.
	fn from(path: &PathBuf) -> Self {
		if path.is_dir() { Self::None }
		else {
			match imghdr::from_file(path) {
				Ok(Some(imghdr::Type::Png)) => Self::Png,
				Ok(Some(imghdr::Type::Jpeg)) => Self::Jpeg,
				_ => Self::None,
			}
		}
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
