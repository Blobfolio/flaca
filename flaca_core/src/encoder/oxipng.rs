/*!
# Flaca: OXIPNG
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



/// OXIPNG.
#[derive(Debug, Clone, Copy)]
pub struct Oxipng {}

impl super::Encoder for Oxipng {
	/// The binary file name.
	const BIN: &'static str = "oxipng";
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Png;
	/// The program name.
	const NAME: &'static str = "Oxipng";
	/// The program URL.
	const URL: &'static str = "https://github.com/shssoichiro/oxipng";

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		Command::new(crate::OXIPNG.clone())
			.args(&[
				"-s",
				"-q",
				"-a",
				"-t",
				"1",
				"--fix",
				"-o",
				"3",
				"-i",
				"0",
				path.as_ref().to_str().unwrap_or(""),
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

		Ok(())
	}
}
