/*!
# Flaca: Images!
*/

mod jpegtran;
mod kind;
mod zopflipng;



use kind::ImageKind;
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
	size: u64,
}

impl<'a> FlacaImage<'a> {
	/// # New.
	///
	/// Create a wrapper for a raw image. If the image can't be loaded or is
	/// an unsupported format — e.g. [`ImageKind::Jpeg`] when `jpeg` is false —
	/// `None` is returned.
	pub(super) fn new(file: &'a Path, jpeg: bool, png: bool) -> Option<Self> {
		// Try to load the data.
		let data = fs::read(file).ok()?;
		if data.is_empty() { None }
		else {
			let kind = match ImageKind::parse(data.as_slice())? {
				ImageKind::Jpeg if jpeg => ImageKind::Jpeg,
				ImageKind::Png if png => ImageKind::Png,
				_ => return None,
			};

			let size = u64::try_from(data.len()).ok()?;
			Some(Self {
				file,
				kind,
				data,
				size,
			})
		}
	}
}

impl FlacaImage<'_> {
	#[must_use]
	/// # Compress.
	///
	/// This method will run the lossless compression pass(es) against the
	/// source image and save the result if it winds up smaller.
	///
	/// A tuple containing the original file size and the new file size is
	/// returned. If the two values are equal, no savings occurrred.
	pub(super) fn compress(&mut self) -> (u64, u64) {
		match self.kind {
			ImageKind::Jpeg => { self.mozjpeg(); },
			ImageKind::Png => {
				self.oxipng();
				self.zopflipng();
			},
		}

		// The buffer can't be empty, so if it is smaller than the original
		// size, savings happened!
		let after = self.data.len() as u64;
		if after < self.size && write_atomic::write_file(self.file, &self.data).is_ok() {
			(self.size, after)
		}
		else { (self.size, self.size) }
	}

	#[inline]
	/// # Compress w/ `MozJPEG`.
	///
	/// The result is comparable to running:
	/// ```bash
	/// jpegtran -copy none -optimize -progressive
	/// ```
	fn mozjpeg(&mut self) { jpegtran::jpegtran_mem(&mut self.data); }

	/// # Compress w/ `Oxipng`
	///
	/// The result is comparable to calling:
	/// ```bash
	/// oxipng -o 3 -s -a -i 0 --fix
	/// ```
	fn oxipng(&mut self) {
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

		if let Ok(mut new) = oxipng::optimize_from_memory(&self.data, &OPTS) {
			// Is it worth saving?
			if
				! new.is_empty() &&
				new.len() < self.data.len() &&
				Some(ImageKind::Png) == ImageKind::parse(&new)
			{
				std::mem::swap(&mut self.data, &mut new);
			}
		}
	}

	#[inline]
	/// # Compress w/ `Zopflipng`.
	///
	/// The result is comparable to calling:
	/// ```bash
	/// zopflipng -m
	/// ```
	fn zopflipng(&mut self) { zopflipng::zopflipng_optimize(&mut self.data); }
}
