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

	#[allow(clippy::cast_possible_truncation)] // It was usize to begin with.
	#[allow(unused_assignments)]
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
		let res = jpegtran::compress(
			self.data.as_ptr(),
			self.size,
			&mut out_ptr,
			&mut out_size,
		);

		if 0 < out_size && ! out_ptr.is_null() {
			// Maybe replace the buffer with our new image!
			if res {
				if let Ok(size) = usize::try_from(out_size) {
					if size < self.size as usize {
						let tmp = unsafe { std::slice::from_raw_parts(out_ptr, size) };
						if Some(ImageKind::Jpeg) == ImageKind::parse(tmp) {
							self.data.truncate(size);
							self.data.copy_from_slice(tmp);
						}
					}
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

	#[allow(unused_assignments)]
	/// # Compress w/ `Zopflipng`.
	///
	/// The result is comparable to calling:
	/// ```bash
	/// zopflipng -m
	/// ```
	fn zopflipng(&mut self) {
		let data_size = self.data.len();
		let src_size = match c_ulong::try_from(data_size) {
			Ok(s) => s,
			Err(_) => return,
		};

		let mut out_ptr = std::ptr::null_mut();
		let mut out_size: c_ulong = 0;

		// Try to compress!
		let res: bool = 0 == unsafe {
			zopflipng::CZopfliPNGOptimize(
				self.data.as_ptr(),
				src_size,
				&zopflipng::CZopfliPNGOptions::default(),
				0, // false
				&mut out_ptr,
				&mut out_size,
			)
		};

		if 0 < out_size && ! out_ptr.is_null() {
			// Maybe replace the buffer with our new image!
			if res {
				if let Ok(size) = usize::try_from(out_size) {
					if size < data_size {
						let tmp = unsafe { std::slice::from_raw_parts(out_ptr, size) };
						if Some(ImageKind::Png) == ImageKind::parse(tmp) {
							self.data.truncate(size);
							self.data.copy_from_slice(tmp);
						}
					}
				}
			}

			// Manually free the C memory.
			unsafe { libc::free(out_ptr.cast::<c_void>()); }
			out_ptr = std::ptr::null_mut();
		}
	}
}
