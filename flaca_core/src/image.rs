/*!
# Flaca: Image Business
*/

use crate::{
	jpegtran,
	Result
};
use std::{
	ffi::OsStr,
	fs,
	io::Write,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};
use tempfile::NamedTempFile;



#[derive(Debug, Clone, Copy, Hash, PartialEq)]
/// Image Kind.
pub enum ImageKind {
	/// Jpeg.
	Jpeg,
	/// Png.
	Png,
	/// Some other kind of file.
	None,
}

impl From<&PathBuf> for ImageKind {
	/// # From `PathBuf`.
	///
	/// This determines the "true" image type by evaluating the Magic MIME
	/// headers of the file, if any.
	fn from(path: &PathBuf) -> Self {
		if path.is_dir() { Self::None }
		else {
			match imghdr::from_file(path).ok().flatten() {
				Some(imghdr::Type::Png) => Self::Png,
				Some(imghdr::Type::Jpeg) => Self::Jpeg,
				_ => Self::None,
			}
		}
	}
}

impl ImageKind {
	#[must_use]
	/// # Extension.
	///
	/// This returns an appropriate file extension for the image type,
	/// lowercase and unambiguous.
	///
	/// While flaca avoids renaming source files, it does occasionally need to
	/// rename the temporary files it is working on as some dependencies crash
	/// if a file has the wrong extension.
	pub const fn ext(&self) -> &str {
		match self {
			Self::Png => ".png",
			Self::Jpeg => ".jpg",
			Self::None => "",
		}
	}

	/// # Tempfile.
	///
	/// Make a temporary file with the correct file extension. See also
	/// [`ImageKind::ext`].
	pub fn mktmp(self) -> Result<NamedTempFile> {
		if self == Self::None { return Err(()); }

		tempfile::Builder::new()
			.suffix(OsStr::new(self.ext()))
			.tempfile()
			.map_err(|_| ())
	}

	/// # Tempfile w/ Data.
	///
	/// Make a temporary file and seed it with the contents of `data`. This is
	/// like a temporary copy operation except that ownership and permissions
	/// might be different for the tempfile.
	///
	/// For our purposes, permissions don't really matter so long as flaca
	/// itself can read/write. Data is held in an agnostic byte vector between
	/// runs.
	pub fn mktmp_with(self, data: &[u8]) -> Result<NamedTempFile> {
		let target = self.mktmp()?;
		let mut file = target.as_file();
		file.write_all(data)
			.and_then(|_| file.flush())
			.map(|_| target)
			.map_err(|_| ())
	}
}

#[allow(unused_must_use)]
/// # Compress!
///
/// Losslessly compress the image at `path`, updating the original file if
/// savings happen to be found.
///
/// JPEG images are passed through `MozJPEG`, stripping markers and optimizing
/// the encoding.
///
/// PNG images are passed through `Oxipng` and `Zopflipng` (in that order) to
/// brute-force the best possible encoding. During the process, markers are
/// stripped and interlacing is removed.
///
/// See the encoder-specific methods for additional details.
pub fn compress(path: &PathBuf) {
	// This should be a valid image type.
	let kind = ImageKind::from(path);
	if kind == ImageKind::None { return; }

	// Pull the starting data, and make sure it has some!
	let mut data: Vec<u8> = fs::read(path).unwrap_or_default();
	let len: usize = data.len();
	if len == 0 { return; }

	// JPEGs only need to run through MozJPEG.
	if kind == ImageKind::Jpeg { compress_mozjpeg(&mut data); }
	// PNGs are passed through Oxipng first. If that doesn't crash, we'll send
	// files to Zopflipng for potentially more compression.
	else if compress_oxipng(&mut data).is_ok() {
		compress_zopflipng(&mut data);
	}

	// Write changes back to the original file, if any.
	if data.len() < len {
		let _ = tempfile_fast::Sponge::new_for(path)
			.and_then(|mut out| out.write_all(&data).and_then(|_| out.commit()));
	}
}

/// # Compress: `MozJpeg`
///
/// The result is comparable to running:
/// ```bash
/// jpegtran -copy none -optimize -progressive
/// ```
pub fn compress_mozjpeg(data: &mut Vec<u8>) -> Result<()> {
	// This pass can be done without needless file I/O! Hurray!
	let tmp: Vec<u8> = unsafe { jpegtran::jpegtran_mem(data)? };

	// Update the source if applicable.
	if tmp.len() < data.len() {
		data.truncate(tmp.len());
		data[..].copy_from_slice(&tmp[..]);
		Ok(())
	}
	else { Err(()) }
}

/// # Compress: `Oxipng`
///
/// Pass the in-memory PNG data to `Oxipng` to see what savings it can come up
/// with. If `Oxipng` is unable to parse/fix the file, an `Err` is returned (so
/// we can pack up and go home early). Otherwise an `Ok()` response is
/// returned, `true` indicating savings happened, `false` indicating no savings
/// happened.
///
/// The result is comparable to calling:
/// ```bash
/// oxipng -o 3 -s -a -i 0 --fix
/// ```
pub fn compress_oxipng(data: &mut Vec<u8>) -> Result<bool> {
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
	let tmp: Vec<u8> = oxipng::optimize_from_memory(
		data,
		&OPTS
	).map_err(|_| ())?;

	// Update the source if applicable.
	if ! tmp.is_empty() && tmp.len() < data.len() {
		data.truncate(tmp.len());
		data[..].copy_from_slice(&tmp[..]);
		Ok(true)
	}
	else { Ok(false) }
}

/// # Compress: `Zopflipng`
///
/// This method spawns an external call to the `zopflipng` executable bundled
/// with flaca. If for some reason that file is missing, this compression pass
/// is skipped.
///
/// This approach is less than ideal for a number of reasons, but to date there
/// isn't a workable Rust port of `zopflipng`. `Oxipng` has basic `zopfli`
/// support, but with major performance and compression loss relative to
/// calling an external `zopflipng` on a second pass.
///
/// If/when that situation changes, flaca will internalize the operations!
pub fn compress_zopflipng(data: &mut Vec<u8>) -> Result<()> {
	use std::os::unix::fs::PermissionsExt;

	lazy_static::lazy_static! {
		static ref ZOPFLIPNG: bool = std::fs::metadata("/var/lib/flaca/zopflipng")
			.ok()
			.filter(std::fs::Metadata::is_file)
			.map_or(false, |m| m.permissions().mode() & 0o111 != 0);
	}

	// Abort if Zopflipng is not found.
	if ! *ZOPFLIPNG { return Err(()); }

	// Convert it to a file.
	let target = ImageKind::Png.mktmp_with(data)?;
	let path = target.path()
		.to_str()
		.filter(|x| ! x.is_empty())
		.ok_or(())?;

	// Execute the linked program.
	Command::new("/var/lib/flaca/zopflipng")
		.args(&[
			"-m",
			"-y",
			path,
			path,
		])
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output()
		.map_err(|_| ())?;

	// To see what changed, we need to open and read the file we just wrote.
	// Sucks to have the unnecessary file I/O, but this is still much more
	// efficient than using Rust Zopfli bindings directly.
	let tmp: Vec<u8> = fs::read(path).map_err(|_| ())?;
	drop(target);

	if ! tmp.is_empty() && tmp.len() < data.len() {
		data.truncate(tmp.len());
		data[..].copy_from_slice(&tmp[..]);
	}

	Ok(())
}
