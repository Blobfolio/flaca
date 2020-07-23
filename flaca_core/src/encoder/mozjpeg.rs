/*!
# Flaca: `MozJPEG`
*/

use crate::image::ImageKind;
use crate::jpegtran;
use fyi_witcher::Result;
use std::path::{
	Path,
	PathBuf,
};



/// `MozJPEG`.
#[derive(Debug, Clone, Copy)]
pub struct Mozjpeg {}

impl super::Encoder for Mozjpeg {
	/// The binary file name.
	const BIN: &'static str = "jpegtran";
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Jpeg;
	/// The program name.
	const NAME: &'static str = "MozJPEG";
	/// The program URL.
	const URL: &'static str = "https://github.com/mozilla/mozjpeg";

	/// Find it.
	///
	/// MozJPEG is built-in, so we only need to find ourselves. Really not even
	/// that, but it is good to have for completeness.
	fn find() -> Result<PathBuf> {
		Ok(std::env::current_exe().expect("Flaca should exist!"))
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		if jpegtran::jpegtran(path) { Ok(()) }
		else { Err("Compression failed.".to_string()) }
	}
}
