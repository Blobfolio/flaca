/*!
# Flaca: OXIPNG
*/

use crate::image::ImageKind;
use fyi_witcher::Result;
use oxipng::{
	AlphaOptim,
	Deflaters,
	Headers,
	InFile,
	Options,
	OutFile,
};
use std::path::{
	Path,
	PathBuf,
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

	/// Find it.
	///
	/// Oxipng is built-in, so we only need to find ourselves. Really not even
	/// that, but it is good to have for completeness.
	fn find() -> Result<PathBuf> {
		Ok(std::env::current_exe().expect("Flaca should exist!"))
	}

	/// Encode for Real.
	fn _encode<P> (path: P) -> Result<()>
	where P: AsRef<Path> {
		lazy_static::lazy_static! {
			// The options will remain the same for each run.
			static ref OPTS: Options = {
				let mut o: Options = Options::from_preset(3);

				// Alpha optimizations.
				o.alphas.insert(AlphaOptim::Black);
				o.alphas.insert(AlphaOptim::Down);
				o.alphas.insert(AlphaOptim::Left);
				o.alphas.insert(AlphaOptim::Right);
				o.alphas.insert(AlphaOptim::Up);
				o.alphas.insert(AlphaOptim::White);

				// The alternative deflater seems to perform the same or better
				// than the default, so I guess that's what we're going to use!
				o.deflate = Deflaters::Libdeflater;

				// Fix errors when possible.
				o.fix_errors = true;

				// Strip interlacing.
				o.interlace.replace(0);

				// Strip what can be safely stripped.
				o.strip = Headers::Safe;

				o
			};
		}

		let file_in = InFile::Path(path.as_ref().to_path_buf());
		let file_out = OutFile::Path(None);

		oxipng::optimize(
			&file_in,
			&file_out,
			&OPTS
		).map_err(|e| e.to_string())?;

		Ok(())
	}
}
