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

use crate::find_executable;
use crate::image::ImageKind;
use fyi_core::misc::numbers;
use fyi_core::witcher::formats::FYIFormats;
use fyi_core::witcher::ops::FYIOps;
use fyi_core::witcher::props::FYIProps;
use std::path::{Path, PathBuf};



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
	fn find() -> Result<PathBuf, String> {
		match find_executable(Self::BIN) {
			Some(p) => Ok(p),
			_ => Err(format!("Unable to find {}.", Self::NAME)),
		}
	}

	/// Encode.
	fn encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path> {
		// Get the starting size.
		let before: u64 = path.as_ref().fyi_file_size();
		if 0 == before {
			return Err(format!("Empty file: {}", path.as_ref().fyi_to_string()));
		}

		// Copy it somewhere temporary.
		let out = path.as_ref().fyi_copy_tmp()?;

		// Do the actual encoding.
		Self::_encode(&out)?;

		let after: u64 = out.fyi_file_size();
		if 0 != numbers::saved(before, after) {
			out.fyi_move(&path)?;
		}
		else {
			out.fyi_delete()?;
		}

		Ok(())
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path>;
}
