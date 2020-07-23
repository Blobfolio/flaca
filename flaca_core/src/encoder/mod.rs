/*!
# Flaca: Encoders
*/

mod jpegoptim;
mod mozjpeg;
mod oxipng;
mod pngout;
mod zopflipng;

pub use self::jpegoptim::Jpegoptim;
pub use self::mozjpeg::Mozjpeg;
pub use self::oxipng::Oxipng;
pub use self::pngout::Pngout;
pub use self::zopflipng::Zopflipng;

use crate::{
	find_executable,
	image::ImageKind,
};
use fyi_witcher::{
	Result,
	traits::WitchIO,
	utility::file_size
};
use std::path::{
	Path,
	PathBuf,
};



/// Image Encoder.
pub trait Encoder: Sized {
	/// The binary file name.
	const BIN: &'static str;
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Jpeg;
	/// The program name.
	const NAME: &'static str;
	/// The program URL.
	const URL: &'static str;

	#[must_use]
	/// Does it exist?
	fn exists() -> bool { true }

	/// Find it.
	fn find() -> Result<PathBuf> {
		find_executable(Self::BIN)
			.ok_or_else(|| format!("Could not find {}.", Self::NAME))
	}

	/// Encode.
	fn encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		if ! Self::exists() {
			return Err(Self::_missing());
		}

		// Get the starting size.
		let before: u64 = file_size(path.as_ref());
		if 0 == before {
			return Err(format!("Unable to encode {:?}.", path.as_ref().to_path_buf()));
		}

		// Copy it somewhere temporary.
		let out = path.as_ref().witch_copy_tmp(Self::KIND.suffix())?;

		// Do the actual encoding.
		if let Err(e) = Self::_encode(&out.path()) {
			out.close().map_err(|e| e.to_string())?;
			return Err(e);
		}

		let after: u64 = file_size(out.path());
		if 0 == crate::bytes_saved(before, after) {
			out.close().map_err(|e| e.to_string())?;
		}
		else {
			out.persist(path.as_ref()).map_err(|e| e.to_string())?;
		}

		Ok(())
	}

	#[must_use]
	/// Missing.
	fn _missing() -> String {
		format!("Missing: {} <{}>", Self::NAME, Self::URL)
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path>;
}
