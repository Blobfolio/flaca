/*!
# Flaca: JPEGOPTIM
*/

use crate::image::ImageKind;
use std::{
	path::Path,
	process::{
		Command,
		Stdio,
	},
};



/// JPEGOPTIM.
#[derive(Debug, Clone, Copy)]
pub struct Jpegoptim {}

impl super::Encoder for Jpegoptim {
	/// The binary file name.
	const BIN: &'static str = "jpegoptim";
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Jpeg;
	/// The program name.
	const NAME: &'static str = "jpegoptim";
	/// The program URL.
	const URL: &'static str = "https://github.com/tjko/jpegoptim";

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path> {
		Command::new(crate::JPEGOPTIM.clone())
			.args(&[
				"-q",
				"-f",
				"--strip-all",
				"--all-progressive",
				path.as_ref().to_str().unwrap_or(""),
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output().map_err(|x| x.to_string())?;

		Ok(())
	}
}
