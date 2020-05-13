/*!
# Flaca: Image Business
*/

use crate::encoder::{
	Encoder,
	Jpegoptim,
	Mozjpeg,
	Oxipng,
	Pngout,
	Zopflipng,
};
use fyi_witcher::Result;
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
				let _ = Jpegoptim::encode(&self).is_ok();
				let _ = Mozjpeg::encode(&self).is_ok();
				Ok(())
			},
			ImageKind::Png => {
				let _ = Pngout::encode(&self).is_ok();
				let _ = Oxipng::encode(&self).is_ok();
				let _ = Zopflipng::encode(&self).is_ok();
				Ok(())
			},
			ImageKind::None => Err(format!("{:?} is not a valid image.", self.to_path_buf())),
		}
	}

	/// Image type.
	fn flaca_image_type(&self) -> ImageKind {
		if self.is_file() {
			match imghdr::from_file(&self) {
				Ok(Some(imghdr::Type::Png)) => ImageKind::Png,
				Ok(Some(imghdr::Type::Jpeg)) => ImageKind::Jpeg,
				_ => ImageKind::None,
			}
		}
		else {
			ImageKind::None
		}
	}
}
