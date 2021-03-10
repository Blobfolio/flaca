/*!
# Flaca: Lib
*/

use crate::{
	FlacaError,
	ImageKind,
};
use std::{
	convert::TryFrom,
	ffi::OsStr,
	fs,
	io::Write,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};



#[derive(Debug)]
/// # Flaca Image.
///
/// This struct holds the state of a given image, updating the file if
/// compression yields any savings.
pub struct FlacaImage<'a> {
	file: &'a PathBuf,
	kind: ImageKind,
	data: Vec<u8>,
}

impl<'a> TryFrom<&'a PathBuf> for FlacaImage<'a> {
	type Error = FlacaError;

	fn try_from(file: &'a PathBuf) -> Result<Self, Self::Error> {
		// Load the image data.
		let data = fs::read(file).map_err(|_| FlacaError::ReadFail)?;
		if data.is_empty() { Err(FlacaError::EmptyFile) }
		else {
			Ok(Self {
				file,
				kind: ImageKind::try_from(data.as_slice())?,
				data,
			})
		}
	}
}

impl FlacaImage<'_> {
	/// # Compress.
	///
	/// ## Errors
	///
	/// This method returns an error if there are issues compressing the file
	/// (other than cases where no savings were possible).
	pub fn compress(&mut self) -> Result<(), FlacaError> {
		let changed: bool = match self.kind {
			ImageKind::Jpeg => {
				self.mozjpeg()?
			},
			ImageKind::Png => {
				let a: bool = self.oxipng()?;
				self.zopflipng()? || a
			},
		};

		// Save the newer, smaller version!
		if changed {
			tempfile_fast::Sponge::new_for(self.file)
				.and_then(|mut out| out.write_all(&self.data).and_then(|_| out.commit()))
				.map_err(|_| FlacaError::WriteFail)?;
		}

		Ok(())
	}

	#[inline]
	/// # Compress w/ `MozJPEG`.
	///
	/// The result is comparable to running:
	/// ```bash
	/// jpegtran -copy none -optimize -progressive
	/// ```
	///
	/// ## Errors
	///
	/// This method returns an error if there are issues compressing the file
	/// (other than cases where no savings were possible).
	fn mozjpeg(&mut self) -> Result<bool, FlacaError> {
		Ok(self.maybe_update(&unsafe { super::jpegtran::jpegtran_mem(&self.data)? }))
	}

	/// # Compress w/ `Oxipng`
	///
	/// Pass the in-memory PNG data to `Oxipng` to see what savings it can come
	/// up with. If `Oxipng` is unable to parse/fix the file, an `Err` is
	/// returned (so we can pack up and go home early).
	///
	/// The result is comparable to calling:
	/// ```bash
	/// oxipng -o 3 -s -a -i 0 --fix
	/// ```
	///
	/// ## Errors
	///
	/// This method returns an error if there are issues compressing the file
	/// (other than cases where no savings were possible).
	fn oxipng(&mut self) -> Result<bool, FlacaError> {
		use oxipng::{
			AlphaOptim,
			Deflaters,
			Headers,
			Options,
		};

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
				o.strip = Headers::All;

				o
			};
		}

		// This pass can be done without needless file I/O! Hurray!
		Ok(self.maybe_update(
			&oxipng::optimize_from_memory(&self.data, &OPTS)
				.map_err(|_| FlacaError::ParseFail)?
		))
	}

	/// # Compress w/ `Zopflipng`.
	///
	/// This method spawns an external call to the `zopflipng` executable
	/// bundled with flaca. If for some reason that file is missing, this
	/// compression pass is skipped.
	///
	/// This approach is less than ideal for a number of reasons, but to date
	/// there isn't a workable Rust port of `zopflipng`. `Oxipng` has basic
	/// `zopfli` support, but with major performance and compression loss
	/// relative to calling an external `zopflipng` on a second pass.
	///
	/// If/when that situation changes, flaca will internalize the operations!
	///
	/// ## Errors
	///
	/// This method returns an error if there are issues compressing the file
	/// (other than cases where no savings were possible).
	fn zopflipng(&mut self) -> Result<bool, FlacaError> {
		use std::os::unix::fs::PermissionsExt;

		lazy_static::lazy_static! {
			static ref ZOPFLIPNG: bool = fs::metadata("/var/lib/flaca/zopflipng")
				.ok()
				.filter(fs::Metadata::is_file)
				.map_or(false, |m| m.permissions().mode() & 0o111 != 0);
		}

		// Abort if Zopflipng is not found or executable.
		if ! *ZOPFLIPNG { return Ok(false); }

		// Make a tempfile copy we can throw at Zopflipng.
		let target = tempfile::Builder::new()
			.suffix(OsStr::new(".png"))
			.tempfile()
			.map_err(|_| FlacaError::WriteFail)?;

		{
			let mut file = target.as_file();
			file.write_all(&self.data)
				.and_then(|_| file.flush())
				.map_err(|_| FlacaError::WriteFail)?;
		}

		// Pull the tempfile path.
		let path = target.path().as_os_str();

		// Execute the linked program.
		let status = Command::new("/var/lib/flaca/zopflipng")
			.args(&[
				OsStr::new("-m"),
				OsStr::new("-y"),
				path,
				path,
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_err(|_| FlacaError::WriteFail)?;

		if ! status.success() { return Err(FlacaError::WriteFail); }

		// To see what changed, we need to open and read the file we just
		// wrote. Sucks to have the unnecessary file I/O, but this is still
		// much more efficient than using Rust Zopfli bindings directly.
		let changed: bool = self.maybe_update(
			&fs::read(path).map_err(|_| FlacaError::ReadFail)?
		);

		// Explicitly drop the tempfile to make sure it gets cleaned up.
		drop(target);

		Ok(changed)
	}

	/// # Maybe Update Buffer.
	///
	/// This will replace the inline source data with the new version, provided
	/// the new version has length and is smaller than the original.
	fn maybe_update(&mut self, new: &[u8]) -> bool {
		if ! new.is_empty() && new.len() < self.data.len() {
			self.data.truncate(new.len());
			self.data[..].copy_from_slice(new);
			true
		}
		else { false }
	}
}
