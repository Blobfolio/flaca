/*!
# Flaca: Encoders
*/

mod jpegoptim;
mod mozjpeg;
mod oxipng;
mod pngout;
mod zopflipng;

pub use jpegoptim::Jpegoptim;
pub use mozjpeg::Mozjpeg;
pub use oxipng::Oxipng;
pub use pngout::Pngout;
pub use zopflipng::Zopflipng;

use crate::{
	find_executable,
	image::ImageKind,
};
use fyi_core::{
	Error,
	Result,
	traits::{
		MebiSaved,
		PathProps,
	},
};
use fyi_witch::traits::WitchIO;
use std::path::{
	Path,
	PathBuf
};



/// Image Encoder.
pub trait Encoder: Sized {
	/// The binary file name.
	const BIN: &'static str;
	/// Image Kind.
	const KIND: ImageKind;
	/// The program name.
	const NAME: &'static str;
	/// The program URL.
	const URL: &'static str;

	/// Find it.
	fn find() -> Result<PathBuf> {
		find_executable(Self::BIN)
			.ok_or_else(|| Error::new(format!("Could not find {}.", Self::NAME)))
	}

	/// Encode.
	fn encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		// Get the starting size.
		let before: u64 = path.as_ref().file_size();
		if 0 == before {
			return Err(Error::new(format!("Unable to encode {:?}.", path.as_ref().to_path_buf())));
		}

		// Copy it somewhere temporary.
		let out = path.as_ref().witch_copy_tmp(Self::KIND.suffix())?;

		// Do the actual encoding.
		if let Err(e) = Self::_encode(&out.path()) {
			out.close()?;
			return Err(e);
		}

		let after: u64 = out.path().file_size();
		if 0 == before.saved(after) {
			out.close()?;
		}
		else {
			out.persist(path.as_ref())
				.map_err(|e| Error::new(format!("{}", e)))?;
		}

		Ok(())
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path>;
}
