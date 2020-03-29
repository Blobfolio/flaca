/*!
# Flaca: PNGOUT
*/

use crate::image::ImageKind;
use std::path::Path;
use std::process::{Command, Stdio};



/// PNGOUT.
#[derive(Debug, Clone, Copy)]
pub struct Pngout {}

impl super::Encoder for Pngout {
	/// The binary file name.
	const BIN: &'static str = "pngout";
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Png;
	/// The program name.
	const NAME: &'static str = "Pngout";
	/// The program URL.
	const URL: &'static str = "http://advsys.net/ken/utils.htm";

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path> {
		Command::new(crate::PNGOUT.clone())
			.args(&[
				path.as_ref().to_str().unwrap_or(""),
				"-q",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output().map_err(|x| x.to_string())?;

		Ok(())
	}
}
