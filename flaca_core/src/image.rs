/*!
# Flaca: Image Business
*/

use std::fmt;
use std::path::Path;
use fyi_core::witcher::formats::FYIFormats;
use crate::encoder::*;



#[derive(Debug, Clone, Copy)]
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



/// Image Type.
pub trait ImagePath {
	/// Encode.
	fn flaca_encode(&self) -> Result<(), String>;

	/// Image type.
	fn flaca_image_type(&self) -> ImageKind;
}

impl ImagePath for Path {
	/// Encode.
	fn flaca_encode(&self) -> Result<(), String> {
		match self.flaca_image_type() {
			ImageKind::Jpeg => {
				Jpegoptim::encode(&self)?;
				Mozjpeg::encode(&self)?;
				Ok(())
			},
			ImageKind::Png => {
				Pngout::encode(&self)?;
				Oxipng::encode(&self)?;
				Zopflipng::encode(&self)?;
				Ok(())
			},
			ImageKind::None => Err(format!("Invalid image: {}", self.fyi_to_string())),
		}
	}

	/// Image type.
	fn flaca_image_type(&self) -> ImageKind {
		match self.is_file() {
			true => match imghdr::from_file(&self) {
				Ok(Some(imghdr::Type::Png)) => ImageKind::Png,
				Ok(Some(imghdr::Type::Jpeg)) => ImageKind::Jpeg,
				_ => ImageKind::None,
			},
			false => ImageKind::None,
		}
	}
}
