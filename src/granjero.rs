// Flaca: Granjero
//
// Encoders and results.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use crate::lugar::Lugar;
use std::fmt;
use std::io::{Error, ErrorKind};
use std::time::SystemTime;
use std::path::PathBuf;
use std::process::Command;

#[derive(Copy, Clone, Debug, PartialEq)]
/// Image type.
pub enum Tipo {
	Jpg,
	Png,
}

impl fmt::Display for Tipo {
	/// Display format.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Tipo::Jpg => write!(f, "{}", "jpg"),
			Tipo::Png => write!(f, "{}", "png"),
		}
	}
}

impl Tipo {
	/// Deduce an image type from a file extension.
	pub fn from(path: Lugar) -> Result<Tipo, Error> {
		let ext = path.extension()?.to_string().to_lowercase();

		if "jpg" == ext || "jpeg" == ext {
			return Ok(Tipo::Jpg);
		}
		else if "png" == ext {
			return Ok(Tipo::Png);
		}

		Err(Error::new(ErrorKind::InvalidInput, "Not an image."))
	}
}

#[derive(Clone, Debug, PartialEq)]
/// An image encoder.
pub enum Granjero {
	Jpegoptim(Option<Lugar>),
	Mozjpeg(Option<Lugar>),
	Oxipng(Option<Lugar>),
	Pngout(Option<Lugar>),
	Zopflipng(Option<Lugar>),
}

impl fmt::Display for Granjero {
	/// Display format.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self.cmd_path().unwrap_or("".to_string()))
	}
}

impl Granjero {
	/// Extract the path to the binary.
	///
	/// I am surely missing something; there has got to be a less rote
	/// way of extracting the () part.
	pub fn path(&self) -> Lugar {
		match self.to_owned() {
			Granjero::Jpegoptim(p) => {
				if let Some(x) = p {
					x
				} else {
					Lugar::Path("".into())
				}
			},
			Granjero::Mozjpeg(p) => {
				if let Some(x) = p {
					x
				} else {
					Lugar::Path("".into())
				}
			},
			Granjero::Oxipng(p) => {
				if let Some(x) = p {
					x
				} else {
					Lugar::Path("".into())
				}
			},
			Granjero::Pngout(p) => {
				if let Some(x) = p {
					x
				} else {
					Lugar::Path("".into())
				}
			},
			Granjero::Zopflipng(p) => {
				if let Some(x) = p {
					x
				} else {
					Lugar::Path("".into())
				}
			},
		}
	}

	/// The command path, as a string.
	pub fn cmd_path(&self) -> Result<String, Error> {
		lazy_static! {
			static ref paths: Vec<String> = format!("/usr/share/flaca:{}", std::env::var("PATH").unwrap_or("".to_string()))
				.split(":")
				.map(String::from)
				.collect();
		}

		// We'll need to reference the command name a handful of times.
		let cmd_name = self.cmd_name();

		// Return a user-specific path if it exists.
		if let Some(path) = match *self {
			Granjero::Jpegoptim(ref p) => { p },
			Granjero::Mozjpeg(ref p) => { p },
			Granjero::Oxipng(ref p) => { p },
			Granjero::Pngout(ref p) => { p },
			Granjero::Zopflipng(ref p) => { p },
		} {
			if path.is_file() && path.name().unwrap_or("".to_string()) == self.cmd_name() {
				if let Ok(y) = path.canonical() {
					return Ok(y);
				}
			}
		}

		// Check for a right-named file in the right places.
		for p in paths.iter() {
			let mut x = Lugar::Path(format!("{}/{}", p, cmd_name).into());
			if x.is_file() && x.name().unwrap_or("".to_string()) == self.cmd_name() {
				if let Ok(y) = x.canonical() {
					return Ok(y);
				}
			}

			// MozJPEG might be hiding in a special place. Let's check
			// this after our share directory.
			if "jpegtran" == cmd_name && "/usr/share/flaca" == p {
				x = Lugar::Path("/opt/mozjpeg/bin/jpegtran".into());
				if x.is_file() {
					if let Ok(y) = x.canonical() {
						return Ok(y);
					}
				}
			}
		}

		Err(Error::new(ErrorKind::NotFound, "Missing encoder."))
	}

	/// In lieu of being able to actually verify file integrity, we can
	/// at least make sure the file name matches what is expected.
	pub fn cmd_name(&self) -> String {
		match *self {
			Granjero::Jpegoptim(_) => "jpegoptim".to_string(),
			Granjero::Mozjpeg(_) => "jpegtran".to_string(),
			Granjero::Oxipng(_) => "oxipng".to_string(),
			Granjero::Pngout(_) => "pngout".to_string(),
			Granjero::Zopflipng(_) => "zopflipng".to_string(),
		}
	}

	/// The program name.
	pub fn name(&self) -> String {
		match *self {
			Granjero::Jpegoptim(_) => "Jpegoptim".to_string(),
			Granjero::Mozjpeg(_) => "MozJPEG".to_string(),
			Granjero::Oxipng(_) => "Oxipng".to_string(),
			Granjero::Pngout(_) => "Pngout".to_string(),
			Granjero::Zopflipng(_) => "Zopflipng".to_string(),
		}
	}

	/// Compress an image using a given encoder.
	pub fn compress(&self, result: &mut Cosecha) -> Result<(), Error> {
		// Verify encoder exists.
		let cmd_path = self.cmd_path()?;

		// Verify the image exists and is an image.
		if ! result.path.is_file() {
			return Err(Error::new(ErrorKind::NotFound, "Missing image."));
		}
		result.update();

		let ext = result.path.extension()?.to_lowercase();
		if ext != "jpeg" && ext != "jpg" && ext != "png" {
			return Err(Error::new(ErrorKind::InvalidInput, "Incorrect file type."));
		}

		// Might as well grab the permissions while we have them.
		let perms = result.path.perms()?;
		let owner = result.path.owner()?;
		let working1 = result.path.clone(None, None)?;
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
			Granjero::Mozjpeg(_) => {
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
			let tmp = Lugar::Path(working2.into());
			tmp.migrate(PathBuf::from(working1.canonical()?), None, None)?;
		}

		// How is working1 looking?
		let end_size: u64 = working1.size()?;
		if end_size > 0 && result.end_size > 0 && end_size < result.end_size {
			working1.migrate(PathBuf::from(result.path.canonical()?), Some(perms), Some(owner))?;
		}

		result.update();
		Ok(())
	}
}

#[derive(Debug)]
/// A compression result.
pub struct Cosecha {
	pub path: Lugar,
	pub start_time: SystemTime,
	pub start_size: u64,
	pub end_time: SystemTime,
	pub end_size: u64,
}

impl Default for Cosecha {
	fn default() -> Cosecha {
		Cosecha {
			path: Lugar::Path("".into()),
			start_time: SystemTime::now(),
			start_size: 0,
			end_time: SystemTime::now(),
			end_size: 0,
		}
	}
}

impl Cosecha {
	/// Open a new result.
	pub fn new(path: Lugar) -> Cosecha {
		let p = path.canonical().unwrap_or("".to_string());

		Cosecha {
			path: Lugar::Path(PathBuf::from(p).to_path_buf()),
			start_size: path.size().unwrap_or(0),
			..Cosecha::default()
		}
	}

	/// Time elapsed since result was started.
	pub fn elapsed(&self) -> u64 {
		if self.start_time != self.end_time {
			if let Ok(y) = self.end_time.duration_since(self.start_time) {
				return y.as_secs();
			}
		}

		0
	}

	/// Amount of space saved since result was first started.
	pub fn saved(&self) -> u64 {
		if self.end_size > 0 && self.start_size > 0 && self.end_size < self.start_size {
			return self.start_size - self.end_size;
		}

		0
	}

	/// Recalculate the end state.
	pub fn update(&mut self) {
		self.end_size = self.path.size().unwrap_or(0);
		self.end_time = SystemTime::now();
	}
}
