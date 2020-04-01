/*!
# Flaca: Zopflipng
*/

use crate::image::ImageKind;
use std::path::Path;
use std::process::{
	Command,
	Stdio,
};



/// Zopflipng.
#[derive(Debug, Clone, Copy)]
pub struct Zopflipng {}

impl super::Encoder for Zopflipng {
	/// The binary file name.
	const BIN: &'static str = "zopflipng";
	/// Image Kind.
	const KIND: ImageKind = ImageKind::Png;
	/// The program name.
	const NAME: &'static str = "Zopflipng";
	/// The program URL.
	const URL: &'static str = "https://github.com/google/zopfli";

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<(), String>
	where P: AsRef<Path> {
		let out = path.as_ref().to_str().unwrap_or("");
		Command::new(crate::ZOPFLIPNG.clone())
			.args(&[
				"-m",
				"-y",
				&out,
				&out,
			])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output().map_err(|x| x.to_string())?;

		Ok(())
	}
}
