/*!
Flaca: Images
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]



use mando::lugar::Lugar;
use std::error::Error;
use std::fmt;
use std::process::{Command, Stdio};
use std::time::SystemTime;



#[derive(Debug, Clone, Copy, PartialEq)]
/// Image kind.
pub enum ImagenKind {
	/// JPEG.
	Jpg,
	/// PNG.
	Png,
	/// Nothing.
	None,
}

#[derive(Debug, Clone)]
/// Image encoder.
pub enum Imagen {
	/// Jpegoptim.
	///
	/// See: https://github.com/tjko/jpegoptim
	Jpegoptim(Lugar),
	/// MozJPEG.
	///
	/// We're specifically targetting MozJPEG because of its compression
	/// gains, however if another jpegtran implementation is installed
	/// instead, it will be used.
	///
	/// See: https://github.com/mozilla/mozjpeg
	MozJPEG(Lugar),
	/// Oxipng
	///
	/// See: https://github.com/shssoichiro/oxipng
	Oxipng(Lugar),
	/// Pngout
	///
	/// Oxipng is touted as a replacement for Pngout, and while it is
	/// much better in almost every way, Pngout has a few proprietary
	/// tricks up its sleeves. So alas, we must run both.
	///
	/// See: http://advsys.net/ken/utils.htm
	Pngout(Lugar),
	/// Zopflipng
	///
	/// This is technically redundant with Oxipng installed, however
	/// performance and compression are currently much better in
	/// natively-built Zopfli binaries than the Rust port used by Oxi.
	///
	/// https://github.com/google/zopfli
	Zopflipng(Lugar),
}

#[derive(Debug)]
/// Compression results for a given image.
pub struct Cosecha {
	/// Image path.
	path: Lugar,
	/// Start time.
	start_time: SystemTime,
	/// Start size in bytes.
	start_size: u64,
	/// End time (i.e. compression jobs finished).
	end_time: SystemTime,
	/// End size in bytes.
	end_size: u64,
}



impl Default for Cosecha {
	fn default() -> Cosecha {
		Cosecha {
			path: Lugar::None,
			start_time: SystemTime::now(),
			start_size: 0,
			end_time: SystemTime::now(),
			end_size: 0,
		}
	}
}

impl Cosecha {
	// -----------------------------------------------------------------
	// Init/Tests
	// -----------------------------------------------------------------

	/// Open a new result.
	pub fn new(path: Lugar) -> Cosecha {
		// Initialize with the path assuming it exists and is an image.
		if
			path.is_file() &&
			(
				path.has_extension("jpg") ||
				path.has_extension("jpeg") ||
				path.has_extension("png")
			)
		{
			let size = path.size().unwrap_or(0);

			return Cosecha {
				path: path,
				start_size: size,
				..Cosecha::default()
			};
		}

		Cosecha::default()
	}

	/// Is Image
	///
	/// Double-check the supplied path belongs to an image.
	pub fn is_image(&self) -> bool {
		self.as_image_kind() != ImagenKind::None
	}

	/// As ImagenKind
	///
	/// Given the path, which ImagenKind would it be?
	pub fn as_image_kind(&self) -> ImagenKind {
		if self.path.is_file() {
			if self.path.has_extension("jpg") || self.path.has_extension("jpeg") {
				return ImagenKind::Jpg;
			}
			else if self.path.has_extension("png") {
				return ImagenKind::Png;
			}
		}

		ImagenKind::None
	}

	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Path.
	pub fn path(&self) -> Lugar {
		self.path.clone()
	}

	/// Start time.
	pub fn start_time(&self) -> SystemTime {
		self.start_time
	}

	/// Start size in bytes.
	pub fn start_size(&self) -> u64 {
		self.start_size
	}

	/// End time.
	pub fn end_time(&self) -> SystemTime {
		self.end_time
	}

	/// End size in bytes.
	pub fn end_size(&self) -> u64 {
		self.end_size
	}

	/// Time elapsed.
	pub fn elapsed(&self) -> u64 {
		Lugar::time_diff(self.start_time, self.end_time).unwrap_or(0)
	}

	/// Total saved in bytes.
	pub fn saved(&self) -> u64 {
		if
			self.start_size > 0 &&
			self.end_size > 0 &&
			self.end_size < self.start_size
		{
			return self.start_size - self.end_size;
		}

		0
	}

	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Log Results to File.
	pub fn log(&self, log: &Lugar) -> Result<(), Box<dyn std::error::Error>> {
		// Not supposed to be logging?
		if ! log.is_some() || log.is_dir() {
			return Ok(());
		}

		// Can't log?
		if ! self.path.is_file() || 0 == self.start_size || 0 == self.end_size {
			return Err("Could not gather the result information.".into());
		}

		// Save computed values to variables.
		let path: String = self.path.path()?;
		let elapsed: u64 = self.elapsed();
		let saved: u64 = self.saved();

		// Format a human-readable log message.
		let msg: String =
			if saved > 0 {
				format!("Saved {} bytes in {} seconds.", saved, elapsed)
			}
			else {
				"No change.".into()
			};

		// Send it on its way!
		log.append(format!(
			"{} \"{}\" -- {} {} {} -- {}",
			Lugar::local_now().to_rfc3339(),
			path,
			self.start_size,
			self.end_size,
			elapsed,
			msg
		))?;

		Ok(())
	}

	/// Update result.
	///
	/// All values are computed; this just recomputes them.
	pub fn update(&mut self) {
		if self.is_image() {
			self.end_size = self.path.size().unwrap_or(0);
			self.end_time = SystemTime::now();
		}
	}
}



/// Display
///
/// Convert Imagen enums into a human-friendly encoder name.
impl fmt::Display for Imagen {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Imagen::Jpegoptim(_) => write!(f, "{}", "Jpegoptim"),
			Imagen::MozJPEG(_) => write!(f, "{}", "MozJPEG"),
			Imagen::Oxipng(_) => write!(f, "{}", "Oxipng"),
			Imagen::Pngout(_) => write!(f, "{}", "Pngout"),
			Imagen::Zopflipng(_) => write!(f, "{}", "Zopflipng"),
		}
	}
}

impl Imagen {
	// -----------------------------------------------------------------
	// Tests
	// -----------------------------------------------------------------

	/// Whether the encoder has a valid path.
	pub fn is_some(&self) -> bool {
		self.bin_path().is_file()
	}

	/// As ImagenKind
	///
	/// Which ImagenKind is this encoder for?
	pub fn as_image_kind(&self) -> ImagenKind {
		match *self {
			Imagen::Jpegoptim(_) => ImagenKind::Jpg,
			Imagen::MozJPEG(_) => ImagenKind::Jpg,
			Imagen::Oxipng(_) => ImagenKind::Png,
			Imagen::Pngout(_) => ImagenKind::Png,
			Imagen::Zopflipng(_) => ImagenKind::Png,
		}
	}

	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Binary name.
	pub fn bin_name(&self) -> String {
		match *self {
			Imagen::Jpegoptim(_) => "jpegoptim".to_string(),
			Imagen::MozJPEG(_) => "jpegtran".to_string(),
			Imagen::Oxipng(_) => "oxipng".to_string(),
			Imagen::Pngout(_) => "pngout".to_string(),
			Imagen::Zopflipng(_) => "zopflipng".to_string(),
		}
	}

	/// Binary path.
	pub fn bin_path(&self) -> Lugar {
		match *self {
			Imagen::Jpegoptim(_) => self.__jpegoptim_path(),
			Imagen::MozJPEG(_) => self.__mozjpeg_path(),
			Imagen::Oxipng(_) => self.__oxipng_path(),
			Imagen::Pngout(_) => self.__pngout_path(),
			Imagen::Zopflipng(_) => self.__zopflipng_path(),
		}
	}

	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Compress an image with the encoder.
	pub fn compress(&self, result: &mut Cosecha) -> Result<(), Box<dyn Error>> {
		// Make sure we have the encoder.
		let cmd_path = self.bin_path();
		if cmd_path.is_none() {
			return Err(format!("Missing encoder {}.", self.bin_name()).into());
		}
		let cmd_path = cmd_path.path()?;

		// Make sure we still have an image.
		if ! result.is_image() {
			return Err("The image has vanished!".into());
		}
		// And the right kind of image.
		else if result.as_image_kind() != self.as_image_kind() {
			return Err("Wrong image format for encoder.".into());
		}

		// Might as well update the results.
		result.update();

		// Pull ownership of original file and generate working copies.
		let perms = result.path.perms()?;
		let owner = result.path.owner()?;
		let mut working1: Lugar = result.path.tmp_cp()?;
		let working1_str: String = working1.path()?;
		let mut working2: Lugar = Lugar::None;
		let mut working2_str: String = "".to_string();

		// Build a command.
		let mut com = Command::new(cmd_path);
		match *self {
			Imagen::Jpegoptim(_) => {
				com.arg("-q");
				com.arg("-f");
				com.arg("--strip-all");
				com.arg("--all-progressive");
				com.arg(&working2_str);
			},
			Imagen::MozJPEG(_) => {
				working2_str = format!("{}.bak", working1_str);
				working2 = Lugar::new(&working2_str);

				com.arg("-copy");
				com.arg("none");
				com.arg("-optimize");
				com.arg("-progressive");
				com.arg("-outfile");
				com.arg(&working2_str);
				com.arg(&working1_str);
			},
			Imagen::Oxipng(_) => {
				com.arg("-s");
				com.arg("-q");
				com.arg("--fix");
				com.arg("-o");
				com.arg("6");
				com.arg("-i");
				com.arg("0");
				com.arg(&working1_str);
			},
			Imagen::Pngout(_) => {
				com.arg(&working1_str);
				com.arg("-q");
			},
			Imagen::Zopflipng(_) => {
				working2_str = format!("{}.bak", working1_str);
				working2 = Lugar::new(&working2_str);

				com.arg("-m");
				com.arg(&working1_str);
				com.arg(&working2_str);
			},
		}

		// Run the command!
		com
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

		// Deal with working2 if it exists.
		if working2.is_some() {
			working2.mv(&mut working1, None, None)?;
		}

		// Replace original, if needed.
		let end_size: u64 = working1.size()?;
		if end_size > 0 && result.end_size > 0 && end_size < result.end_size {
			working1.mv(&mut result.path, Some(perms), Some(owner))?;
		}
		// Remove working file.
		else if working1.is_file() {
			if let Err(_) = working1.rm() {}
		}

		// Reupdate result.
		result.update();

		Ok(())
	}

	// -----------------------------------------------------------------
	// Inner Helpers
	// -----------------------------------------------------------------

	/// Return the inner path as a reference.
	fn __inner_ref(&self) -> &Lugar {
		match *self {
			Imagen::Jpegoptim(ref x) => x,
			Imagen::MozJPEG(ref x) => x,
			Imagen::Oxipng(ref x) => x,
			Imagen::Pngout(ref x) => x,
			Imagen::Zopflipng(ref x) => x,
		}
	}

	/// Jpegoptim binary path.
	///
	/// The user-specified path is given priority, but Flaca will also
	/// check other executable paths.
	fn __jpegoptim_path(&self) -> Lugar {
		lazy_static! {
			static ref JPEGOPTIM: Lugar = {
				// Try the Flaca share directory first.
				let mut tmp: Lugar = Lugar::Path(format!("/usr/share/flaca/{}", "jpegoptim").into());
				if ! tmp.is_file() {
					tmp = Lugar::executable_path("jpegoptim");
				}

				tmp
			};
		}

		// Prefer a user-supplied path.
		if let Ok(x) = self.__inner_ref().name() {
			if self.__inner_ref().is_file() && "jpegoptim" == x {
				return Lugar::new(self.__inner_ref().as_path_buf());
			}
		}

		if JPEGOPTIM.is_file() {
			return Lugar::new(JPEGOPTIM.as_path_buf());
		}

		Lugar::None
	}

	/// MozJPEG binary path.
	///
	/// The user-specified path is given priority, but Flaca will also
	/// check other executable paths.
	fn __mozjpeg_path(&self) -> Lugar {
		lazy_static! {
			static ref MOZJPEG: Lugar = {
				// Try the Flaca share directory first.
				let mut tmp: Lugar = Lugar::Path(format!("/usr/share/flaca/{}", "jpegtran").into());
				if ! tmp.is_file() {
					// Next try the sideload MozJPEG path.
					tmp = Lugar::Path("/opt/mozjpeg/bin/jpegtran".into());
					// And lastly, anything executable.
					if ! tmp.is_file() {
						tmp = Lugar::executable_path("jpegtran");
					}
				}

				tmp
			};
		}

		// Prefer a user-supplied path.
		if let Ok(x) = self.__inner_ref().name() {
			if self.__inner_ref().is_file() && "jpegtran" == x {
				return Lugar::new(self.__inner_ref().as_path_buf());
			}
		}

		if MOZJPEG.is_file() {
			return Lugar::new(MOZJPEG.as_path_buf());
		}

		Lugar::None
	}

	/// Oxipng binary path.
	///
	/// The user-specified path is given priority, but Flaca will also
	/// check other executable paths.
	fn __oxipng_path(&self) -> Lugar {
		lazy_static! {
			static ref OXIPNG: Lugar = {
				// Try the Flaca share directory first.
				let mut tmp: Lugar = Lugar::Path(format!("/usr/share/flaca/{}", "oxipng").into());
				if ! tmp.is_file() {
					tmp = Lugar::executable_path("oxipng");
				}

				tmp
			};
		}

		// Prefer a user-supplied path.
		if let Ok(x) = self.__inner_ref().name() {
			if self.__inner_ref().is_file() && "oxipng" == x {
				return Lugar::new(self.__inner_ref().as_path_buf());
			}
		}

		if OXIPNG.is_file() {
			return Lugar::new(OXIPNG.as_path_buf());
		}

		Lugar::None
	}

	/// Pngout binary path.
	///
	/// The user-specified path is given priority, but Flaca will also
	/// check other executable paths.
	fn __pngout_path(&self) -> Lugar {
		lazy_static! {
			static ref PNGOUT: Lugar = {
				// Try the Flaca share directory first.
				let mut tmp: Lugar = Lugar::Path(format!("/usr/share/flaca/{}", "pngout").into());
				if ! tmp.is_file() {
					tmp = Lugar::executable_path("pngout");
				}

				tmp
			};
		}

		// Prefer a user-supplied path.
		if let Ok(x) = self.__inner_ref().name() {
			if self.__inner_ref().is_file() && "pngout" == x {
				return Lugar::new(self.__inner_ref().as_path_buf());
			}
		}

		if PNGOUT.is_file() {
			return Lugar::new(PNGOUT.as_path_buf());
		}

		Lugar::None
	}

	/// Zopflipng binary path.
	///
	/// The user-specified path is given priority, but Flaca will also
	/// check other executable paths.
	fn __zopflipng_path(&self) -> Lugar {
		lazy_static! {
			static ref ZOPFLIPNG: Lugar = {
				// Try the Flaca share directory first.
				let mut tmp: Lugar = Lugar::Path(format!("/usr/share/flaca/{}", "zopflipng").into());
				if ! tmp.is_file() {
					tmp = Lugar::executable_path("zopflipng");
				}

				tmp
			};
		}

		// Prefer a user-supplied path.
		if let Ok(x) = self.__inner_ref().name() {
			if self.__inner_ref().is_file() && "zopflipng" == x {
				return Lugar::new(self.__inner_ref().as_path_buf());
			}
		}

		if ZOPFLIPNG.is_file() {
			return Lugar::new(ZOPFLIPNG.as_path_buf());
		}

		Lugar::None
	}
}
