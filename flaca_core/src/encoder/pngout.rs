/*!
# Flaca: PNGOUT
*/

use crate::image::ImageKind;
use fyi_core::Result;
use std::{
	path::Path,
	process::{
		Command,
		Stdio,
	},
};



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
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		Command::new(crate::PNGOUT.clone())
			.args(&[
				path.as_ref().to_str().unwrap_or(""),
				"-q",
				"-y",
				"-force",
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

		Ok(())
	}
}
