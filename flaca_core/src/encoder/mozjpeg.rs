/*!
# Flaca: MozJPEG
*/

use crate::image::ImageKind;
use fyi_core::witcher::props::FYIProps;
use std::path::{
	Path,
	PathBuf,
};
use std::process::{
	Command,
	Stdio,
};



/// MozJPEG.
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
	/// MozJPEG uses the same binary name as jpegtran, so we need to
	/// look for it in a specific place.
	fn find() -> Result<PathBuf, String> {
		let path: PathBuf = PathBuf::from("/opt/mozjpeg/bin/jpegtran");
		match path.fyi_is_executable() {
			true => Ok(path),
			_ => Err(format!("Unable to find {}.", Self::NAME)),
		}
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path> {
		let out = path.as_ref().to_str().unwrap_or("");
		Command::new(crate::MOZJPEG.clone())
			.args(&[
				"-copy",
				"none",
				"-optimize",
				"-progressive",
				"-outfile",
				&out,
				&out,
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output().map_err(|x| x.to_string())?;

		Ok(())
	}
}
