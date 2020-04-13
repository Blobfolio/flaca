/*!
# Flaca: Image Business
*/

use crate::encoder::*;
use fyi_core::{
	Error,
	Result,
};
use std::{
	fmt,
	path::Path,
};



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
	fn flaca_encode(&self) -> Result<()>;

	/// Image type.
	fn flaca_image_type(&self) -> ImageKind;
}

impl ImagePath for Path {
	/// Encode.
	fn flaca_encode(&self) -> Result<()> {
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
			ImageKind::None => Err(Error::PathInvalid(self.to_path_buf(), "is not an image")),
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
