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
	traits::path::{
		FYIPath,
		FYIPathIO,
	},
	util::numbers,
};
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
			.ok_or(Error::PathInvalid(PathBuf::from(Self::BIN), "not found"))
	}

	/// Encode.
	fn encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		// Get the starting size.
		let before: u64 = path.as_ref().fyi_file_size();
		if 0 == before {
			return Err(Error::PathRead(path.as_ref().to_path_buf()));
		}

		// Copy it somewhere temporary.
		let out = path.as_ref().fyi_copy_tmp(Self::KIND.suffix())?;

		// Do the actual encoding.
		Self::_encode(&out.path())?;

		let after: u64 = out.path().fyi_file_size();
		if 0 != numbers::saved(before, after) {
			out.persist(path.as_ref())?;
		}
		else {
			drop(out);
		}

		Ok(())
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path>;
}
