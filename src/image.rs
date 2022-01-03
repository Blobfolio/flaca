/*!
# Flaca: Lib
*/

use crate::ImageKind;
use once_cell::sync::Lazy;
use std::{
	fs,
	path::Path,
};



#[derive(Debug)]
/// # Flaca Image.
///
/// This struct holds the state of a given image, updating the file if
/// compression yields any savings.
pub(super) struct FlacaImage<'a> {
	file: &'a Path,
	kind: ImageKind,
	data: Vec<u8>,
}

impl<'a> FlacaImage<'a> {
	/// # New.
	///
	/// Create a wrapper for a raw image.
	///
	/// ## Errors
	///
	/// This will return an error if the image is unreadable or invalid.
	pub(super) fn new(file: &'a Path) -> Option<Self> {
		// Try to load the data.
		let data = fs::read(file).ok()?;
		if data.is_empty() { None }
		// Return the result!
		else {
			Some(Self {
				file,
				kind: ImageKind::parse(data.as_slice())?,
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
	pub(super) fn compress(&mut self) -> bool {
		let changed: bool = match self.kind {
			ImageKind::Jpeg => self.mozjpeg(),
			ImageKind::Png => {
				let a: bool = self.oxipng();
				let b: bool = self.zopflipng();
				a || b
			},
		};

		// Save the newer, smaller version!
		if changed {
			write_atomic::write_file(self.file, &self.data).is_ok()
		}
		else { false }
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
	fn mozjpeg(&mut self) -> bool {
		unsafe { super::jpegtran::jpegtran_mem(&self.data) }
			.map_or(
				false,
				|new| self.maybe_update(&new)
			)
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
	fn oxipng(&mut self) -> bool {
		use oxipng::{
			AlphaOptim,
			Deflaters,
			Headers,
			Options,
		};

		static OPTS: Lazy<Options> = Lazy::new(|| {
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
		});

		// This pass can be done without needless file I/O! Hurray!
		oxipng::optimize_from_memory(&self.data, &OPTS)
			.map_or(
				false,
				|new| self.maybe_update(&new)
			)
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
	fn zopflipng(&mut self) -> bool {
		super::zopflipng::zopflipng_optimize(&self.data)
			.map_or(
				false,
				|new| self.maybe_update(&new)
			)
	}

	/// # Maybe Update Buffer.
	///
	/// This will replace the inline source data with the new version, provided
	/// the new version has length and is smaller than the original.
	fn maybe_update(&mut self, new: &[u8]) -> bool {
		if
			! new.is_empty() &&
			new.len() < self.data.len() &&
			ImageKind::parse(new).map_or(false, |k| k == self.kind)
		{
			self.data.truncate(new.len());
			self.data[..].copy_from_slice(new);
			true
		}
		else { false }
	}
}
