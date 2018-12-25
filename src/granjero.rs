// Flaca: Granjero
//
// Encoders, results, jobs, etc.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use crate::lugar::Lugar;
use std::io::{Error, ErrorKind};
use std::process::Command;
use std::time::SystemTime;

/// A path to use when an encoder is missing.
pub const NO_COMMAND: &str = "/dev/null";

#[derive(Debug, Clone, PartialEq)]
/// Image encoders.
pub enum Granjero {
	Jpegoptim(Lugar),
	MozJPEG(Lugar),
	Oxipng(Lugar),
	Pngout(Lugar),
	Zopflipng(Lugar),
}

impl Granjero {
	// -----------------------------------------------------------------
	// Init/Conversion
	// -----------------------------------------------------------------

	/// As String.
	pub fn as_string(&self) -> String {
		self.name()
	}

	// -----------------------------------------------------------------
	// State
	// -----------------------------------------------------------------

	/// Whether or not the encoder (probably) exists.
	pub fn is(&self) -> bool {
		self.__inner().is_file() && self.__inner().has_name(self.bin_name())
	}

	/// Is this encoder for JPEG images?
	pub fn is_for_jpg(&self) -> bool {
		match *self {
			Granjero::Jpegoptim(_) => true,
			Granjero::MozJPEG(_) => true,
			_ => false,
		}
	}

	/// Is this encoder for PNG images?
	pub fn is_for_png(&self) -> bool {
		match *self {
			Granjero::Oxipng(_) => true,
			Granjero::Pngout(_) => true,
			Granjero::Zopflipng(_) => true,
			_ => false,
		}
	}

	// -----------------------------------------------------------------
	// Data
	// -----------------------------------------------------------------

	/// Return the inner Lugar.
	fn __inner(&self) -> &Lugar {
		match *self {
			Granjero::Jpegoptim(ref p) => p,
			Granjero::MozJPEG(ref p) => p,
			Granjero::Oxipng(ref p) => p,
			Granjero::Pngout(ref p) => p,
			Granjero::Zopflipng(ref p) => p,
		}
	}

	/// Return the inner Lugar mutably.
	fn __inner_mut(&mut self) -> &mut Lugar {
		match *self {
			Granjero::Jpegoptim(ref mut p) => p,
			Granjero::MozJPEG(ref mut p) => p,
			Granjero::Oxipng(ref mut p) => p,
			Granjero::Pngout(ref mut p) => p,
			Granjero::Zopflipng(ref mut p) => p,
		}
	}

	/// Return the path. Similar to command, but it will always match
	/// something.
	pub fn path(&self) -> Lugar {
		if let Ok(x) = self.cmd() {
			return Lugar::new(x);
		}

		Lugar::new(NO_COMMAND)
	}

	/// Return the program name.
	pub fn name(&self) -> String {
		match *self {
			Granjero::Jpegoptim(_) => "Jpegoptim".to_string(),
			Granjero::MozJPEG(_) => "MozJPEG".to_string(),
			Granjero::Oxipng(_) => "Oxipng".to_string(),
			Granjero::Pngout(_) => "Pngout".to_string(),
			Granjero::Zopflipng(_) => "Zopflipng".to_string(),
		}
	}

	/// Return the binary file name.
	pub fn bin_name(&self) -> String {
		match *self {
			Granjero::MozJPEG(_) => "jpegtran".to_string(),
			_ => self.name().to_lowercase(),
		}
	}

	/// Return the command.
	///
	/// This checks for the program in the user-supplied path, then the
	/// Flaca shared dir (/usr/share/flaca), and finally any directories
	/// under $PATH. If the program isn't found in any of those, an
	/// error comes back instead.
	pub fn cmd(&self) -> Result<String, Error> {
		// This maybe has a path attached.
		if self.is() {
			if let Ok(x) = self.__inner().canonical() {
				return Ok(x);
			}
		}

		match *self {
			Granjero::Jpegoptim(_) => Granjero::cmd_jpegoptim(),
			Granjero::MozJPEG(_) => Granjero::cmd_mozjpeg(),
			Granjero::Oxipng(_) => Granjero::cmd_oxipng(),
			Granjero::Pngout(_) => Granjero::cmd_pngout(),
			Granjero::Zopflipng(_) => Granjero::cmd_zopflipng(),
		}
	}

	/// Command for Jpegoptim.
	fn cmd_jpegoptim() -> Result<String, Error> {
		lazy_static! {
			static ref path: String = match Granjero::find_cmd("jpegoptim".to_string()) {
				Ok(x) => x,
				Err(_) => "".to_string(),
			};
		}

		if path.len() > 0 {
			return Ok(path.to_string());
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	/// Command for MozJPEG.
	fn cmd_mozjpeg() -> Result<String, Error> {
		lazy_static! {
			static ref path: String =
				// MozJPEG shares a bin name with a shittier encoder.
				// For our purposes, we only want to check two places
				// rather than all $PATH.
				if Lugar::new("/usr/share/flaca/jpegtran").is_file() {
					"/usr/share/flaca/jpegtran".to_string()
				}
				else if Lugar::new("/opt/mozjpeg/bin/jpegtran").is_file() {
					"/opt/mozjpeg/bin/jpegtran".to_string()
				}
				else {
					"".to_string()
				};
		}

		if path.len() > 0 {
			return Ok(path.to_string());
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	/// Command for Oxipng.
	fn cmd_oxipng() -> Result<String, Error> {
		lazy_static! {
			static ref path: String = match Granjero::find_cmd("oxipng".to_string()) {
				Ok(x) => x,
				Err(_) => "".to_string(),
			};
		}

		if path.len() > 0 {
			return Ok(path.to_string());
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	/// Command for Pngout.
	fn cmd_pngout() -> Result<String, Error> {
		lazy_static! {
			static ref path: String = match Granjero::find_cmd("pngout".to_string()) {
				Ok(x) => x,
				Err(_) => "".to_string(),
			};
		}

		if path.len() > 0 {
			return Ok(path.to_string());
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	/// Command for Zopflipng.
	fn cmd_zopflipng() -> Result<String, Error> {
		lazy_static! {
			static ref path: String = match Granjero::find_cmd("zopflipng".to_string()) {
				Ok(x) => x,
				Err(_) => "".to_string(),
			};
		}

		if path.len() > 0 {
			return Ok(path.to_string());
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	/// Reusable wrapper to find a command by name among the usual
	/// places. If the user supplied a path, that is evaluated prior to
	/// calling this.
	fn find_cmd(bin: String) -> Result<String, Error> {
		let bin_dirs = Lugar::bin_dirs();

		for mut i in bin_dirs {
			if i.is_dir() {
				if let Err(_) = i.push(&bin) {
					continue;
				}

				if i.is_file() {
					return Ok(i.canonical()?)
				}
			}
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder.").into())
	}

	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Update the instance path.
	pub fn set_path(&mut self, mut path: Lugar) -> Result<(), Error> {
		// Use the no-command path if the path is bad.
		if ! path.is_file() || ! path.has_name(self.name()) {
			path = Lugar::new(NO_COMMAND);
		}

		*self = match *self {
			Granjero::Jpegoptim(_) => Granjero::Jpegoptim(path),
			Granjero::MozJPEG(_) => Granjero::MozJPEG(path),
			Granjero::Oxipng(_) => Granjero::Oxipng(path),
			Granjero::Pngout(_) => Granjero::Pngout(path),
			Granjero::Zopflipng(_) => Granjero::Zopflipng(path),
		};

		Ok(())
	}

	/// Compress an image.
	pub fn compress(&self, result: &mut Cosecha) -> Result<(), Error> {
		// Verify the command exists.
		let cmd_path = self.cmd()?;
		let mut path = result.path();

		// The image has to exist too and match this encoder type.
		if
			! path.is_image() ||
			(path.is_jpg() && ! self.is_for_jpg()) ||
			(path.is_png() && ! self.is_for_png()) {
			return Err(Error::new(ErrorKind::InvalidInput, "Incorrect file/encoder combination.").into());
		}

		// Might as well refresh the result.
		result.update();

		let perms = path.perms()?;
		let owner = path.owner()?;
		let mut working1 = path.tmp_cp()?;
		let mut working2: String = "".to_string();

		// Of course, every binary is different.
		let mut com = Command::new(cmd_path);
		match *self {
			Granjero::Jpegoptim(_) => {
				com.arg("-q");
				com.arg("-f");
				com.arg("--strip-all");
				com.arg("--all-progressive");
				com.arg(working1.canonical()?);
			},
			Granjero::MozJPEG(_) => {
				working2 = format!("{}.bak", working1.canonical()?);
				com.arg("-copy");
				com.arg("none");
				com.arg("-optimize");
				com.arg("-progressive");
				com.arg("-outfile");
				com.arg(&working2);
				com.arg(working1.canonical()?);
			},
			Granjero::Oxipng(_) => {
				com.arg("-s");
				com.arg("-q");
				com.arg("--fix");
				com.arg("-o");
				com.arg("6");
				com.arg("-i");
				com.arg("0");
				com.arg(working1.canonical()?);
			},
			Granjero::Pngout(_) => {
				com.arg(working1.canonical()?);
				com.arg("-q");
			},
			Granjero::Zopflipng(_) => {
				working2 = format!("{}.bak", working1.canonical()?);
				com.arg("-m");
				com.arg(working1.canonical()?);
				com.arg(&working2);
			},
		}

		// Run the command!
		com
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped())
			.output()?;

		// Deal with working2 if it exists.
		if working2.len() > 0 {
			let mut tmp = Lugar::new(&working2);
			tmp.mv(&mut working1, None, None)?;
		}

		// Replace the original, if needed.
		let end_size: u64 = working1.size()?;
		if end_size > 0 && result.end_size() > 0 && result.end_size() > end_size {
			working1.cp(&mut path, Some(perms), Some(owner))?;
		}

		// Remove the working file.
		if working1.is_file() {
			if let Err(_) = working1.rm() {}
		}

		// Update the results again.
		result.update();

		Ok(())
	}
}

#[derive(Debug)]
/// Image compression result.
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
			path: Lugar::new(NO_COMMAND),
			start_time: SystemTime::now(),
			start_size: 0,
			end_time: SystemTime::now(),
			end_size: 0,
		}
	}
}

impl Cosecha {
	/// Start a new result.
	pub fn start(path: Lugar) -> Cosecha {
		if ! path.is_image() {
			return Cosecha::default();
		}

		Cosecha {
			path: Lugar::new(path.as_path_buf()),
			start_size: path.size().unwrap_or(0),
			..Cosecha::default()
		}
	}

	/// Update end bits.
	///
	/// Results are built over time, so update might be called several
	/// times. It does not imply an end until Flaca actually stops
	/// using it.
	pub fn update(&mut self) {
		self.end_size = self.path.size().unwrap_or(0);
		self.end_time = SystemTime::now();
	}

	/// Return the path.
	pub fn path(&self) -> Lugar {
		Lugar::new(self.path.as_path_buf())
	}

	/// Start time.
	pub fn start_time(&self) -> SystemTime {
		self.start_time
	}

	/// Start size.
	pub fn start_size(&self) -> u64 {
		self.start_size
	}

	/// End time.
	pub fn end_time(&self) -> SystemTime {
		self.end_time
	}

	/// End size.
	pub fn end_size(&self) -> u64 {
		self.end_size
	}

	/// Total saved.
	pub fn saved(&self) -> u64 {
		if self.start_size > 0 && self.end_size > 0 && self.start_size > self.end_size {
			return self.start_size - self.end_size;
		}

		0
	}

	/// Total elapsed.
	pub fn elapsed(&self) -> u64 {
		if self.start_time > self.end_time {
			return Lugar::time_diff(self.end_time, self.start_time).unwrap_or(0);
		}

		0
	}
}
