/*!
# Flaca: Images!
*/

mod jpegtran;
mod kind;
pub(self) mod lodepng;
mod zopflipng;



use kind::ImageKind;
use oxipng::Options as OxipngOptions;
use std::{
	fs,
	os::raw::{
		c_ulong,
		c_void,
	},
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
			let kind =
				if png && ImageKind::is_png(&data) { ImageKind::Png }
				else if jpeg && ImageKind::is_jpeg(&data) { ImageKind::Jpeg }
				else { return None; };

			let size = u64::try_from(data.len()).ok()?;
			Some(Self { file, kind, data, size })
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
	pub(super) fn compress(&mut self, oxi: &OxipngOptions) -> (u64, u64) {
		match self.kind {
			ImageKind::Jpeg => { self.mozjpeg(); },
			ImageKind::Png => {
				self.oxipng(oxi);
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

	#[allow(clippy::cast_possible_truncation, unused_assignments, unsafe_code)]
	/// # Compress w/ `MozJPEG`.
	///
	/// The result is comparable to running:
	/// ```bash
	/// jpegtran -copy none -optimize -progressive
	/// ```
	fn mozjpeg(&mut self) {
		let mut out_ptr = std::ptr::null_mut();
		let mut out_size: c_ulong = 0;

		// Try to compress!
		let res: bool = jpegtran::compress(
			self.data.as_ptr(),
			self.size,
			&mut out_ptr,
			&mut out_size,
		);

		if 0 < out_size && ! out_ptr.is_null() {
			if res && out_size < self.size {
				let tmp = unsafe { std::slice::from_raw_parts(out_ptr, out_size as usize) };
				if ImageKind::is_jpeg(tmp) {
					self.data.truncate(out_size as usize);
					self.data.copy_from_slice(tmp);
				}
			}

			// Manually free the C memory.
			unsafe { libc::free(out_ptr.cast::<c_void>()); }
			out_ptr = std::ptr::null_mut();
		}
	}

	/// # Compress w/ `Oxipng`
	///
	/// The result is comparable to calling:
	/// ```bash
	/// oxipng -o 3 -s -a -i 0 --fix
	/// ```
	fn oxipng(&mut self, opts: &OxipngOptions) {
		if let Ok(mut new) = oxipng::optimize_from_memory(&self.data, opts) {
			if ! new.is_empty() && new.len() < self.data.len() && ImageKind::is_png(&new) {
				std::mem::swap(&mut self.data, &mut new);
			}
		}
	}

	#[allow(clippy::cast_possible_truncation, unused_assignments, unsafe_code)]
	/// # Compress w/ `Zopflipng`.
	///
	/// The result is comparable to calling:
	/// ```bash
	/// zopflipng -m
	/// ```
	fn zopflipng(&mut self) {
		if let Some(mut new) = zopflipng::optimize(&self.data) {
			// This only returns a result if smaller than the source. We just
			// need to verify the output isn't unrecognizably corrupt.
			if ImageKind::is_png(&new) {
				std::mem::swap(&mut self.data, &mut new);
			}
		}
	}
}



#[inline]
/// # Generate Oxipng Options.
///
/// This returns the strongest compression profile available for Oxipng without
/// using its built-in zopfli deflater. (We run the _full_ zopflipng as a
/// separate pass, so there's no benefit to doing it within Oxipng.)
pub(super) fn oxipng_options() -> OxipngOptions {
	use oxipng::{
		AlphaOptim,
		Deflaters,
		Headers,
		IndexSet,
	};

	// This is the configuration for "preset 3", plus:
	// * fix errors
	// * use libdeflater
	// * check all the alphas
	// * strip all headers
	// * disable interlacing
	OxipngOptions {
		backup: false,
		pretend: false,
		fix_errors: true,
		force: false,
		preserve_attrs: false,
		filter: IndexSet::from([0, 1, 2, 3, 4, 5]),
		interlace: Some(0),
		alphas: IndexSet::from([
			AlphaOptim::NoOp,
			AlphaOptim::Black, AlphaOptim::Down, AlphaOptim::Left,
			AlphaOptim::Right, AlphaOptim::Up, AlphaOptim::White,
		]),
		bit_depth_reduction: true,
		color_type_reduction: true,
		palette_reduction: true,
		grayscale_reduction: true,
		idat_recoding: true,
		strip: Headers::All,
		deflate: Deflaters::Libdeflater,
		use_heuristics: false,
		timeout: None,
	}
}
