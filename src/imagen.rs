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
pub enum ImagenKind {
	Jpg,
	Png,
	None,
}

#[derive(Debug, Clone)]
pub enum Imagen {
	Jpegoptim(Lugar),
	MozJPEG(Lugar),
	Oxipng(Lugar),
	Pngout(Lugar),
	Zopflipng(Lugar),
}

#[derive(Debug)]
pub struct Cosecha {
	path: Lugar,
	start_time: SystemTime,
	start_size: u64,
	end_time: SystemTime,
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

	pub fn update(&mut self) {
		if self.is_image() {
			self.end_size = self.path.size().unwrap_or(0);
			self.end_time = SystemTime::now();
		}
	}

	pub fn is_image(&self) -> bool {
		self.as_image_kind() != ImagenKind::None
	}

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

	pub fn path(&self) -> Lugar {
		self.path.clone()
	}

	pub fn start_size(&self) -> u64 {
		self.start_size
	}

	pub fn start_time(&self) -> SystemTime {
		self.start_time
	}

	pub fn end_size(&self) -> u64 {
		self.end_size
	}

	pub fn end_time(&self) -> SystemTime {
		self.end_time
	}

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

	pub fn elapsed(&self) -> u64 {
		Lugar::time_diff(self.start_time, self.end_time).unwrap_or(0)
	}
}



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
	pub fn is_some(&self) -> bool {
		self.bin_path().is_file()
	}

	pub fn bin_name(&self) -> String {
		match *self {
			Imagen::Jpegoptim(_) => "jpegoptim".to_string(),
			Imagen::MozJPEG(_) => "jpegtran".to_string(),
			Imagen::Oxipng(_) => "oxipng".to_string(),
			Imagen::Pngout(_) => "pngout".to_string(),
			Imagen::Zopflipng(_) => "zopflipng".to_string(),
		}
	}

	pub fn bin_path(&self) -> Lugar {
		match *self {
			Imagen::Jpegoptim(_) => self.__jpegoptim_path(),
			Imagen::MozJPEG(_) => self.__mozjpeg_path(),
			Imagen::Oxipng(_) => self.__oxipng_path(),
			Imagen::Pngout(_) => self.__pngout_path(),
			Imagen::Zopflipng(_) => self.__zopflipng_path(),
		}
	}

	pub fn as_image_kind(&self) -> ImagenKind {
		match *self {
			Imagen::Jpegoptim(_) => ImagenKind::Jpg,
			Imagen::MozJPEG(_) => ImagenKind::Jpg,
			Imagen::Oxipng(_) => ImagenKind::Png,
			Imagen::Pngout(_) => ImagenKind::Png,
			Imagen::Zopflipng(_) => ImagenKind::Png,
		}
	}

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

	fn __inner_ref(&self) -> &Lugar {
		match *self {
			Imagen::Jpegoptim(ref x) => x,
			Imagen::MozJPEG(ref x) => x,
			Imagen::Oxipng(ref x) => x,
			Imagen::Pngout(ref x) => x,
			Imagen::Zopflipng(ref x) => x,
		}
	}

	fn __inner_ref_mut(&mut self) -> &mut Lugar {
		match *self {
			Imagen::Jpegoptim(ref mut x) => x,
			Imagen::MozJPEG(ref mut x) => x,
			Imagen::Oxipng(ref mut x) => x,
			Imagen::Pngout(ref mut x) => x,
			Imagen::Zopflipng(ref mut x) => x,
		}
	}

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
