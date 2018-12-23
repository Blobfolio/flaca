// Flaca
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>



#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]



extern crate ansi_term;
extern crate chrono;
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate regex;
extern crate term_size;

use chrono::TimeZone;
use std::io::{Error, ErrorKind, Write};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub mod lugar;
pub mod granjero;
pub mod diario;
pub mod mundo;



// ---------------------------------------------------------------------
// Definitions
// ---------------------------------------------------------------------

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
type Result<T> = std::result::Result<T, Box<Error>>;



// ---------------------------------------------------------------------
// Args
// ---------------------------------------------------------------------

#[derive(Debug)]
/// Options contains all of the runtime settings referenced during a
/// scan.
struct Options {
	debug: bool,
	pretend: bool,
	quiet: bool,
	log: Option<PathBuf>,
	min_age: Option<u64>,
	max_age: Option<u64>,
	min_size: Option<u64>,
	max_size: Option<u64>,
	skip: Option<ImageType>,
	bin_jpegoptim: Result<(Encoder, PathBuf)>,
	bin_mozjpeg: Result<(Encoder, PathBuf)>,
	bin_oxipng: Result<(Encoder, PathBuf)>,
	bin_pngout: Result<(Encoder, PathBuf)>,
	bin_zopflipng: Result<(Encoder, PathBuf)>,
	raw: Vec<Image>,
}

impl Default for Options {
	/// Populate default Options.
	fn default() -> Options {
		let none_jpegoptim: Option<PathBuf> = None;
		let none_mozjpeg: Option<PathBuf> = None;
		let none_oxipng: Option<PathBuf> = None;
		let none_pngout: Option<PathBuf> = None;
		let none_zopflipng: Option<PathBuf> = None;

		Options {
			debug: false,
			pretend: false,
			quiet: false,
			log: None,
			min_age: None,
			max_age: None,
			min_size: None,
			max_size: None,
			skip: None,
			bin_jpegoptim: match Encoder::Jpegoptim.as_path_buf(none_jpegoptim) {
				Ok(x) => Ok((Encoder::Jpegoptim, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Jpegoptim is not installed").into()),
			},
			bin_mozjpeg: match Encoder::Mozjpeg.as_path_buf(none_mozjpeg) {
				Ok(x) => Ok((Encoder::Mozjpeg, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "MozJPEG is not installed").into()),
			},
			bin_oxipng: match Encoder::Oxipng.as_path_buf(none_oxipng) {
				Ok(x) => Ok((Encoder::Oxipng, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Oxipng is not installed").into()),
			},
			bin_pngout: match Encoder::Pngout.as_path_buf(none_pngout) {
				Ok(x) => Ok((Encoder::Pngout, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Pngout is not installed").into()),
			},
			bin_zopflipng: match Encoder::Zopflipng.as_path_buf(none_zopflipng) {
				Ok(x) => Ok((Encoder::Zopflipng, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Zopflipng is not installed").into()),
			},
			raw: Vec::new(),
		}
	}
}

impl Options {
	/// Populate Options from a Clap ArgMatches response.
	fn from(args: &clap::ArgMatches) -> Options {
		// Most of this can be built straight away.
		let mut out: Options = Options {
			debug: args.is_present("debug"),
			pretend: args.is_present("pretend"),
			quiet: args.is_present("quiet"),
			log: match args.value_of("log") {
				Some(x) => Some(std::path::PathBuf::from(x)),
				None => None,
			},
			min_age: match args.value_of("min_age") {
				Some(x) => match x.parse::<u64>() {
					Ok(y) => Some(y * 60),
					Err(_) => None,
				},
				None => None,
			},
			max_age: match args.value_of("max_age") {
				Some(x) => match x.parse::<u64>() {
					Ok(y) => Some(y * 60),
					Err(_) => None,
				},
				None => None,
			},
			min_size: match args.value_of("min_size") {
				Some(x) => match x.parse::<u64>() {
					Ok(y) => Some(y),
					Err(_) => None,
				},
				None => None,
			},
			max_size: match args.value_of("max_size") {
				Some(x) => match x.parse::<u64>() {
					Ok(y) => Some(y),
					Err(_) => None,
				},
				None => None,
			},
			skip: match args.value_of("skip") {
				Some(x) => match x {
					"jpg" => Some(ImageType::Jpg),
					"jpeg" => Some(ImageType::Jpg),
					"png" => Some(ImageType::Png),
					_ => None,
				},
				None => None,
			},
			bin_jpegoptim: match Encoder::Jpegoptim.as_path_buf(args.value_of("bin_jpegoptim")) {
				Ok(x) => Ok((Encoder::Jpegoptim, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Jpegoptim is not installed").into()),
			},
			bin_mozjpeg: match Encoder::Mozjpeg.as_path_buf(args.value_of("bin_mozjpeg")) {
				Ok(x) => Ok((Encoder::Mozjpeg, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "MozJPEG is not installed").into()),
			},
			bin_oxipng: match Encoder::Oxipng.as_path_buf(args.value_of("bin_oxipng")) {
				Ok(x) => Ok((Encoder::Oxipng, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Oxipng is not installed").into()),
			},
			bin_pngout: match Encoder::Pngout.as_path_buf(args.value_of("bin_pngout")) {
				Ok(x) => Ok((Encoder::Pngout, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Pngout is not installed").into()),
			},
			bin_zopflipng: match Encoder::Zopflipng.as_path_buf(args.value_of("bin_zopflipng")) {
				Ok(x) => Ok((Encoder::Zopflipng, x)),
				Err(_) => Err(Error::new(ErrorKind::NotFound, "Zopflipng is not installed").into()),
			},
			..Self::default()
		};

		// Depending on what is installed on the system, we may not be
		// able to process certain image types.
		if Some(ImageType::Jpg) != out.skip && out.bin_jpegoptim.is_err() && out.bin_mozjpeg.is_err() {
			let msg = Box::from(Error::new(ErrorKind::Other, "Cannot process JPEG images; missing dependencies."));

			// No skip is set, so we can skip JPEGs.
			if out.skip.is_none() {
				out.skip = Some(ImageType::Jpg);
				warning(msg);
			}
			// Nothing to process means an error.
			else if Some(ImageType::Png) == out.skip {
				error(msg);
			}
		}
		else if Some(ImageType::Png) != out.skip && out.bin_oxipng.is_err() && out.bin_pngout.is_err() && out.bin_zopflipng.is_err() {
			let msg = Box::from(Error::new(ErrorKind::Other, "Cannot process PNG images; missing dependencies."));

			// No skip is set, so we can skip JPEGs.
			if out.skip.is_none() {
				out.skip = Some(ImageType::Png);
				warning(msg);
			}
			// Nothing to process means an error.
			else if Some(ImageType::Jpg) == out.skip {
				error(msg);
			}
		}

		// Now we need to see if any images map.
		let mut images = out.parse_images(
			args
				.values_of("INPUT")
				.unwrap()
				.collect(),
		);

		// Abort if there were no images.
		if images.len() < 1 {
			error(Box::from(Error::new(ErrorKind::Other, "No qualifying images were found")));
		}

		// Otherwise sort, dedup, and convert!
		images.sort();
		images.dedup();

		out.raw = images
			.iter()
			.map(|x| { Image::from(std::path::PathBuf::from(x)).unwrap_or(Image::default()) })
			.collect();

		out.debug();

		// Done!
		out
	}

	/// Initialize the Clap CLI magic and populate runtime Options from
	/// it.
	fn from_env() -> Options {
		let args = clap::App::new("Flaca")
			.version(VERSION)
			.author("Blobfolio, LLC <hello@blobfolio.com>")
			.about("Losslessly compress the mierda out of JPEG and PNG images.")
			.arg(clap::Arg::with_name("debug")
				.short("d")
				.long("debug")
				.alias("verbose")
				.conflicts_with("quiet")
				.help("Print verbose information to STDOUT.")
			)
			.arg(clap::Arg::with_name("pretend")
				.long("pretend")
				.alias("dry-run")
				.alias("dry_run")
				.help("Conduct a trial run without altering your images.")
			)
			.arg(clap::Arg::with_name("log")
				.short("l")
				.long("log")
				.help("Log image operations to 'flaca.log' in this directory.")
				.takes_value(true)
				.validator(|x| {
					let path = PathBuf::from(x);

					// Main thing is this can't be a directory.
					if ! path.is_dir() {
						return Err("Value must be a directory.".to_string())
					}

					Ok(())
				})
				.value_name("DIRECTORY")
			)
			.arg(clap::Arg::with_name("min_age")
				.long("min_age")
				.alias("min-age")
				.help("Ignore files younger than this.")
				.takes_value(true)
				.validator(|x| {
					if let Ok(y) = x.parse::<u64>() {
						if y > 0 {
							return Ok(());
						}
					}

					Err("Value must be greater than zero.".to_string())
				})
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("max_age")
				.long("max_age")
				.alias("max-age")
				.help("Ignore files older than this.")
				.takes_value(true)
				.validator(|x| {
					if let Ok(y) = x.parse::<u64>() {
						if y > 0 {
							return Ok(());
						}
					}

					Err("Value must be greater than zero.".to_string())
				})
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("min_size")
				.long("min_size")
				.alias("min-size")
				.help("Ignore files smaller than this.")
				.takes_value(true)
				.validator(|x| {
					if let Ok(y) = x.parse::<u64>() {
						if y > 0 {
							return Ok(())
						}
					}

					Err("Value must be greater than zero.".to_string())
				})
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("max_size")
				.long("max_size")
				.alias("max-size")
				.help("Ignore files larger than this.")
				.takes_value(true)
				.validator(|x| {
					if let Ok(y) = x.parse::<u64>() {
						if y > 0 {
							return Ok(());
						}
					}

					Err("Value must be greater than zero.".to_string())
				})
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("quiet")
				.short("q")
				.long("quiet")
				.conflicts_with("debug")
				.help("Suppress STDOUT. This has no effect on errors.")
			)
			.arg(clap::Arg::with_name("skip")
				.short("s")
				.long("skip")
				.help("Skip images of this type.")
				.possible_values(&["jpeg", "jpg", "png"])
				.takes_value(true)
				.value_name("FORMAT")
			)
			.arg(clap::Arg::with_name("bin_jpegoptim")
				.long("bin_jpegoptim")
				.alias("jpegoptim")
				.alias("bin-jpegoptim")
				.help("Alternate binary path for jpegoptim.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("bin_mozjpeg")
				.long("bin_mozjpeg")
				.alias("jpegtran")
				.alias("mozjpeg")
				.alias("bin-jpegtran")
				.alias("bin-mozjpeg")
				.alias("bin_jpegtran")
				.help("Alternate binary path for MozJPEG.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("bin_oxipng")
				.long("bin_oxipng")
				.alias("oxipng")
				.alias("bin-oxipng")
				.help("Alternate binary path for oxipng.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("bin_pngout")
				.long("bin_pngout")
				.alias("pngout")
				.alias("bin-pngout")
				.help("Alternate binary path for pngout.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("bin_zopflipng")
				.long("bin_zopflipng")
				.alias("bin-zopflipng")
				.alias("zopflipng")
				.help("Alternate binary path for zopflipng.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("INPUT")
				.index(1)
				.help("Images(s) to crunch or where to find them.")
				.multiple(true)
				.required(true)
				.use_delimiter(false)
			)
			.after_help("OPTIMIZERS:
    Jpegoptim <https://github.com/tjko/jpegoptim>
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Pngout    <http://advsys.net/ken/utils.htm>
    Zopflipng <https://github.com/google/zopfli>
			")
			.get_matches();

		Options::from(&args)
	}

	/// Recursively find all applicable image files given the paths
	/// passed through CLI.
	///
	/// Results are returned as canonical Strings for easy sorting and
	/// deduplication.
	fn parse_images<P: AsRef<Path>>(&self, files: Vec<P>) -> Vec<String> {
		let mut out = Vec::new();

		for file in files {
			// Recurse directories.
			if file.as_ref().is_dir() {
				let files = file.as_ref()
					.read_dir()
					.unwrap()
					.map(|x| x.unwrap().path().to_owned())
					.collect();
				out.extend(self.parse_images(files));
			}
			// Just a regular old file.
			else if file.as_ref().is_file() {
				// Should be an expandable path.
				if let Ok(path) = get_file_canonical(file.as_ref()) {
					// Check extension first.
					if let Ok(ext) = ImageType::from(file.as_ref()) {
						// Skipping this type.
						if self.skip == Some(ext) {
							continue;
						}

						// Check file size.
						if self.min_size.is_some() || self.max_size.is_some() {
							if let Ok(size) = get_file_size(file.as_ref()) {
								if (self.min_size.is_some() && size < self.min_size.unwrap()) || (self.max_size.is_some() && size > self.max_size.unwrap()) {
									continue;
								}
							} else {
								continue;
							}
						}

						// Check file time.
						if self.min_age.is_some() || self.max_age.is_some() {
							if let Ok(age) = get_file_modified_since(file) {
								if (self.min_age.is_some() && age < self.min_age.unwrap()) || (self.max_age.is_some() && age > self.max_age.unwrap()) {
									continue;
								}
							} else {
								continue;
							}
						}

						out.push(path);
					}
				}
			}
		}

		// Done!
		out
	}

	/// Print any actionable or non-default runtime Options in effect,
	/// but only if --debug was one of them.
	fn debug(&mut self) {
		// This only applies if we are debugging.
		if false == self.debug {
			return;
		}

		notice("Debug enabled; expect verbose output!".to_string());
		notice(format!("Program started at {}.", get_local_now().to_rfc3339()));

		if true == self.pretend {
			notice("Pretending enabled; source images will not be altered.".to_string());
		}

		if let Some(x) = &self.log {
			notice(format!("Logging to '{}/flaca.log'.", get_file_canonical(x.to_path_buf()).unwrap_or(".".to_string())));
		}

		if let Some(x) = &self.skip {
			notice(format!("Skipping {} files.", x));
		}

		if let Some(x) = &self.min_age {
			notice(format!("Skipping files younger than {}.", get_nice_time(*x, false)));
		}

		if let Some(x) = &self.max_age {
			notice(format!("Skipping files older than {}.", get_nice_time(*x, false)));
		}

		if let Some(x) = &self.min_size {
			notice(format!("Skipping files smaller than {}.", get_nice_size(*x)));
		}

		if let Some(x) = &self.max_size {
			notice(format!("Skipping files larger than {}.", get_nice_size(*x)));
		}

		if Some(ImageType::Jpg) != self.skip {
			if let Ok((_, y)) = &self.bin_jpegoptim {
				let path = get_file_canonical(y.to_path_buf()).unwrap_or("MISSING".to_string());
				notice(format!("Found Jpegoptim at {}.", path));
			}

			if let Ok((_, y)) = &self.bin_mozjpeg {
				let path = get_file_canonical(y.to_path_buf()).unwrap_or("MISSING".to_string());
				notice(format!("Found MozJPEG at {}.", path));
			}
		}

		if Some(ImageType::Png) != self.skip {
			if let Ok((_, y)) = &self.bin_oxipng {
				let path = get_file_canonical(y.to_path_buf()).unwrap_or("MISSING".to_string());
				notice(format!("Found Oxipng at {}.", path));
			}

			if let Ok((_, y)) = &self.bin_pngout {
				let path = get_file_canonical(y.to_path_buf()).unwrap_or("MISSING".to_string());
				notice(format!("Found Pngout at {}.", path));
			}

			if let Ok((_, y)) = &self.bin_zopflipng {
				let path = get_file_canonical(y.to_path_buf()).unwrap_or("MISSING".to_string());
				notice(format!("Found Zopflipng at {}.", path));
			}
		}
	}

	/// Count up the total number of images found.
	fn total_images(&self) -> u64 {
		self.raw.len() as u64
	}

	/// Find the disk size gobbled up by all of the images found.
	fn total_image_size(&mut self) -> u64 {
		let mut size: u64 = 0;

		for i in &self.raw {
			size += get_file_size(i.path.to_path_buf()).unwrap_or(0);
		}

		size
	}
}


// ---------------------------------------------------------------------
// Encoders
// ---------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone)]
/// A list of possible image encoders.
enum Encoder {
	Jpegoptim,
	Mozjpeg,
	Oxipng,
	Pngout,
	Zopflipng,
}

impl std::fmt::Display for Encoder {
	/// Format encoder as its app file name.
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_string())
	}
}

impl Encoder {
	/// Format encoder as its app file name.
	fn as_string(&self) -> String {
		match *self {
			Encoder::Jpegoptim => "jpegoptim".to_string(),
			Encoder::Mozjpeg => "jpegtran".to_string(),
			Encoder::Oxipng => "oxipng".to_string(),
			Encoder::Pngout => "pngout".to_string(),
			Encoder::Zopflipng => "zopflipng".to_string(),
		}
	}

	/// Format encoder as its proper app name.
	fn get_nice_name(&self) -> String {
		match *self {
			Encoder::Jpegoptim => "Jpegoptim".to_string(),
			Encoder::Mozjpeg => "MozJPEG".to_string(),
			Encoder::Oxipng => "Oxipng".to_string(),
			Encoder::Pngout => "Pngout".to_string(),
			Encoder::Zopflipng => "Zopflipng".to_string(),
		}
	}

	/// Format encoder as a path to the local binary, if any.
	///
	/// This method accepts a user-specified path, which takes priority
	/// if present and valid.
	fn as_path_buf<P: AsRef<Path>>(&self, custom: Option<P>) -> Result<PathBuf> {
		lazy_static! {
			static ref paths: Vec<String> = format!("/usr/share/flaca:{}", std::env::var("PATH").unwrap_or("".to_string()))
				.split(":")
				.map(String::from)
				.collect();
		}

		// Return user-specified path if it exists and matches the
		// binary name.
		if let Some(x) = custom {
			if x.as_ref().is_file() {
				if let Ok(y) = get_file_name(x.as_ref()) {
					if y == self.as_string() {
						return Ok(x.as_ref().to_path_buf());
					}
				}
			}
		}

		// Look for a right-named binary in PATH.
		for p in paths.iter() {
			let path = format!("{}/{}", p, self.as_string());
			let mut x: PathBuf = std::path::PathBuf::from(&path);
			if x.is_file() {
				if let Ok(y) = get_file_name(&x) {
					if y == self.as_string() {
						return Ok(x.to_path_buf());
					}
				}
			}

			// MozJPEG might be hiding in a special place. Let's check
			// this after our share directory.
			if path == "/usr/share/flaca/jpegtran" {
				x = std::path::PathBuf::from("/opt/mozjpeg/bin/jpegtran");
				if x.is_file() {
					return Ok(x.to_path_buf());
				}
			}
		}

		Err(Error::new(ErrorKind::Other, format!("{} is not installed.", self)).into())
	}
}



// ---------------------------------------------------------------------
// Images
// ---------------------------------------------------------------------

#[derive(Debug, PartialEq, Clone, Copy)]
/// The type of image.
enum ImageType {
	Jpg,
	Png,
}

impl std::fmt::Display for ImageType {
	/// Format image type as a file extension.
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(f, "{}", self.as_string())
	}
}

impl ImageType {
	/// Format image type as a file extension.
	fn as_string(&self) -> String {
		match *self {
			ImageType::Jpg => "jpg".to_string(),
			ImageType::Png => "png".to_string(),
		}
	}

	/// Deduce an image type given a file path.
	fn from<P: AsRef<Path>>(path: P) -> Result<ImageType> {
		lazy_static! {
			static ref expr: regex::Regex = regex::Regex::new(r"\.(?P<ext>jpe?g|png)$").unwrap();
		}

		// Generate a full path.
		let full = get_file_canonical(path);
		if full.is_ok() {
			let lower: String = full.unwrap().to_lowercase();
			if let Some(matches) = expr.captures(&lower) {
				return match &matches["ext"] {
					"jpg" => Ok(ImageType::Jpg),
					"jpeg" => Ok(ImageType::Jpg),
					"png" => Ok(ImageType::Png),
					_ => Err(Error::new(ErrorKind::Other, "Not an image.").into()),
				};
			}
		}

		Err(Error::new(ErrorKind::Other, "Not an image.").into())
	}
}

#[derive(Debug, PartialEq, Clone)]
/// The result from a single encoder run over a single image.
struct ImageResult {
	path: PathBuf,
	start_size: u64,
	start_time: SystemTime,
	end_size: u64,
	end_time: SystemTime,
}

impl ImageResult {
	/// Start a new compression session.
	fn start<P: AsRef<Path>>(path: P) -> Result<ImageResult> {
		if ! path.as_ref().is_file() {
			return Err(Error::new(ErrorKind::NotFound, "Invalid file.").into());
		}

		Ok(ImageResult {
			path: path.as_ref().to_path_buf(),
			start_size: get_file_size(path.as_ref()).unwrap_or(0),
			start_time: SystemTime::now(),
			end_size: 0,
			end_time: SystemTime::now(),
		})
	}

	/// Finish a compressoin session.
	fn finish(&mut self) -> Result<()> {
		if ! self.path.is_file() {
			return Err(Error::new(ErrorKind::NotFound, "Invalid file.").into());
		}

		self.end_size = get_file_size(self.path.to_path_buf())?;
		self.end_time = SystemTime::now();

		Ok(())
	}

	/// Determine how much space was saved.
	fn get_saved(&self) -> Result<u64> {
		if self.end_size > 0 && self.start_size > 0 {
			if self.start_size > self.end_size {
				return Ok(self.start_size - self.end_size);
			}

			return Err(Error::new(ErrorKind::Other, "The image size did not change.").into());
		}

		Err(Error::new(ErrorKind::Other, "The compression operations failed.").into())
	}

	/// Determine how much time elapsed to reach the result.
	fn get_elapsed(&self) -> Result<u64> {
		if self.end_time != self.start_time {
			if let Ok(y) = self.end_time.duration_since(self.start_time) {
				return Ok(y.as_secs());
			}
		}

		Err(Error::new(ErrorKind::Other, "Elapsed time could not be determined.").into())
	}
}

#[derive(Debug, PartialEq, Clone)]
/// An image (by path).
struct Image {
	path: PathBuf,
}

impl Default for Image {
	/// Default image.
	fn default() -> Image {
		Image {
			path: PathBuf::new(),
		}
	}
}

impl Image {
	/// Initialize an image from a path.
	fn from<P: AsRef<Path>>(path: P) -> Result<Image> {
		let _ext = get_image_type(path.as_ref())?;
		Ok(Image {
			path: path.as_ref().to_path_buf(),
			..Image::default()
		})
	}

	/// Whether or not this image exists.
	fn exists(&self) -> bool {
		self.path.exists()
	}

	/// Create a working clone of the image to manipulate with an
	/// encoder.
	///
	/// We want to avoid working on a source directly so as to mitigate
	/// corruption in the event of an encoder failure, as well as try to
	/// maintain the original metadata (like modification time) unless
	/// we actually have a change worth implementing.
	fn working(&self) -> Result<String> {
		if ! self.exists() {
			return Err(Error::new(ErrorKind::NotFound, "Missing image.").into());
		}

		let ext = get_image_type(&self.path)?;
		let name = get_file_name(&self.path)?;
		let dir = get_file_canonical(std::env::temp_dir())?;
		let mut num: u64 = 0;

		// Guess at a likely unique name.
		let mut out_name: String = format!(
			"{}/{}.__flaca{}.{}",
			dir,
			name,
			num.to_string(),
			ext
		);

		// Repeat until we have something unique.
		while Path::new(&out_name).exists() {
			num += 1;

			out_name = format!(
				"{}/{}.__flaca{}.{}",
				dir,
				name,
				num.to_string(),
				ext
			);
		}

		// Copy the file.
		if let Err(_) = copy_file(self.path.to_path_buf(), PathBuf::from(&out_name).to_path_buf(), None, None) {
			return Err(Error::new(ErrorKind::NotFound, "A working copy could not be created.").into());
		}

		Ok(out_name)
	}

	/// Compress an image with a given encoder at a given path.
	fn compress<P: AsRef<Path>>(&self, encoder: Encoder, bin: P, replace: bool) -> Result<ImageResult> {
		let mut result = ImageResult::start(self.path.to_path_buf())?;

		// Make sure the binary path is still valid.
		if ! bin.as_ref().is_file() {
			return Err(Error::new(ErrorKind::NotFound, format!("Missing encoder: {}.", encoder)).into());
		}

		let perms = get_file_perms(&self.path)?;
		let owner = get_file_owner(&self.path)?;
		let bin_path: String = get_file_canonical(bin)?;
		let working1: String = self.working()?;
		let mut working2: String = "".to_string();

		// Each encoder has its own arbitrary argument setup.
		let mut com = std::process::Command::new(bin_path);
		match encoder {
			Encoder::Jpegoptim => {
				com.arg("-q");
				com.arg("-f");
				com.arg("--strip-all");
				com.arg("--all-progressive");
				com.arg(&working1);
			}
			Encoder::Mozjpeg => {
				working2 = format!("{}.bak", working1);
				com.arg("-copy");
				com.arg("none");
				com.arg("-optimize");
				com.arg("-progressive");
				com.arg("-outfile");
				com.arg(&working2);
				com.arg(&working1);
			}
			Encoder::Oxipng => {
				com.arg("-s");
				com.arg("-q");
				com.arg("--fix");
				com.arg("-o");
				com.arg("6");
				com.arg("-i");
				com.arg("0");
				com.arg(&working1);
			}
			Encoder::Pngout => {
				com.arg(&working1);
				com.arg("-q");
			}
			Encoder::Zopflipng => {
				working2 = format!("{}.bak", working1);
				com.arg("-m");
				com.arg(&working1);
				com.arg(&working2);
			}
		}

		// Run the command!
		if let Err(_) = com
			.stdout(std::process::Stdio::piped())
			.stderr(std::process::Stdio::piped())
			.output() {
			return Err(Error::new(ErrorKind::Other, format!("Encoder failed: {}.", encoder)).into());
		}

		// Move working2 over working1 if it exists.
		if working2.len() > 0 {
			// Push data to working1 since that is what most programs use.
			if let Err(_) = copy_file(PathBuf::from(&working2), PathBuf::from(&working1), Some(perms.to_owned()), Some(owner.to_owned())) {
				return Err(Error::new(ErrorKind::Other, format!("Encoder failed: {}.", encoder)).into());
			}

			// Try to remove working2 for cleanliness.
			if let Err(_) = std::fs::remove_file(PathBuf::from(&working2)) {
				warning(Error::new(ErrorKind::Other, "Some working files may not have been removed.").into());
			}
		}

		// See where we stand.
		result.finish()?;
		result.end_size = get_file_size(PathBuf::from(&working1))?;

		// Replace the original?
		if true == replace && result.get_saved().is_ok() {
			if let Err(_) = copy_file(PathBuf::from(&working1), self.path.to_path_buf(), Some(perms.to_owned()), Some(owner.to_owned())) {
				return Err(Error::new(ErrorKind::Other, format!("Encoder failed: {}.", encoder)).into());
			}
		}

		// Remove the working file.
		if let Err(_) = std::fs::remove_file(PathBuf::from(&working1)) {
			warning(Error::new(ErrorKind::Other, "Some working files may not have been removed.").into());
		}

		Ok(result)
	}
}



// ---------------------------------------------------------------------
// Paths
// ---------------------------------------------------------------------

/// Return a canonical path as a String.
fn get_file_canonical<P: AsRef<Path>>(path: P) -> Result<String> {
	let x = path.as_ref().canonicalize()?;;
	Ok(format!("{}", x.display()))
}

/// Return a file's modification time.
fn get_file_modified<P: AsRef<Path>>(path: P) -> Result<SystemTime> {
	let x = path.as_ref().metadata()?.modified()?;
	Ok(x)
}

/// Return a file's relative (to now) modification time in seconds.
fn get_file_modified_since<P: AsRef<Path>>(path: P) -> Result<u64> {
	let x = get_file_modified(path)?;
	let now = SystemTime::now();
	if let Ok(y) = now.duration_since(x) {
		return Ok(y.as_secs());
	}

	Err(Error::new(ErrorKind::NotFound, "Could not get file modification time.").into())
}

/// Return a file's name.
fn get_file_name<P: AsRef<Path>>(path: P) -> Result<String> {
	if let Some(x) = path.as_ref().file_name() {
		if let Some(y) = std::ffi::OsStr::to_str(x) {
			return Ok(y.to_string());
		}
	}

	Err(Error::new(ErrorKind::NotFound, "Could not get file name.").into())
}

/// Return a file's User Id and Group Id.
fn get_file_owner<P: AsRef<Path>>(path: P) -> Result<(nix::unistd::Uid, nix::unistd::Gid)> {
	let x = path.as_ref().metadata()?;
	Ok((
		nix::unistd::Uid::from_raw(x.uid()),
		nix::unistd::Gid::from_raw(x.gid()),
	))
}

/// Return a file's permissions.
fn get_file_perms<P: AsRef<Path>>(path: P) -> Result<std::fs::Permissions> {
	let x = path.as_ref().metadata()?;
	Ok(x.permissions())
}

/// Return a file's size in bytes.
fn get_file_size<P: AsRef<Path>>(path: P) -> Result<u64> {
	let x = path.as_ref().metadata()?;
	Ok(x.len())
}

/// Return a file's ImageType.
fn get_image_type<P: AsRef<Path>>(path: P) -> Result<ImageType> {
	ImageType::from(path.as_ref())
}

/// Convert a byte size into a more human-friendly unit, like 2.4MB.
fn get_nice_size(size: u64) -> String {
	if size <= 0 {
		return "0B".to_string();
	}

	// Gigabytes.
	if size >= 943718400 {
		return format!("{:.*}GB", 2, size as f64 / 1073741824 as f64);
	}
	// Megabytes.
	else if size >= 921600 {
		return format!("{:.*}MB", 2, size as f64 / 1048576 as f64);
	}
	// Kilobytes.
	else if size >= 900 {
		return format!("{:.*}KB", 2, size as f64 / 1024 as f64);
	}

	format!("{}B", size)
}

/// Copy a file, and optionally override permissions and ownership.
fn copy_file<P: AsRef<Path>>(
	from: P,
	to: P,
	perms: Option<std::fs::Permissions>,
	owner: Option<(nix::unistd::Uid, nix::unistd::Gid)>
) -> Result<()> {
	// Source has to exist.
	if ! from.as_ref().is_file() {
		return Err(Error::new(ErrorKind::NotFound, "Missing source file.").into());
	}

	// If destination is a file, remove it to prevent collisions.
	if to.as_ref().is_file() {
		if let Err(_) = std::fs::remove_file(to.as_ref().to_path_buf()) {
			return Err(Error::new(ErrorKind::AlreadyExists, "Destination already exists and cannot be replaced.").into());
		}
	}
	// Destination cannot be a directory.
	else if to.as_ref().is_dir() {
		return Err(Error::new(ErrorKind::InvalidInput, "Destination cannot be a directory.").into());
	}

	// Try to copy.
	if let Err(_) = std::fs::copy(from.as_ref().to_path_buf(), to.as_ref().to_path_buf()) {
		return Err(Error::new(ErrorKind::Other, "A working copy could not be made.").into());
	}

	// Set permissions?
	if let Some(x) = perms {
		if let Err(_) = std::fs::set_permissions(to.as_ref(), x) {
			warning(Error::new(ErrorKind::Other, "File permissions could not be set.").into());
			return Ok(())
		}
	}

	// Set owner?
	if let Some((uid, gid)) = owner {
		if let Err(_) = nix::unistd::chown(to.as_ref(), Some(uid), Some(gid)) {
			warning(Error::new(ErrorKind::Other, "File permissions could not be set.").into());
			return Ok(())
		}
	}

	Ok(())
}


// ---------------------------------------------------------------------
// Dates and Time
// ---------------------------------------------------------------------

/// Get a datetime object in the local timezone.
fn get_local_now() -> chrono::DateTime<chrono::Local> {
	let start = SystemTime::now();
	let start_since = start.duration_since(std::time::UNIX_EPOCH).expect("Time is meaningless.");

	chrono::Local.timestamp(start_since.as_secs() as i64, 0)
}

/// Format seconds either as a 00:00:00 counter or broken out into a
/// list of hours, minutes, etc.
fn get_nice_time(time: u64, short: bool) -> String {
	if time <= 0 {
		if true == short {
			return "00:00:00".to_string();
		}

		return "0 seconds".to_string();
	}

	// Drill down to days, hours, minutes, and seconds.
	let mut s: u64 = time;
	let d: u64 = ((s / 86400) as f64).floor() as u64;
	s -= d * 86400;
	let h: u64 = ((s / 3600) as f64).floor() as u64;
	s -= h * 3600;
	let m: u64 = ((s / 60) as f64).floor() as u64;
	s -= m * 60;

	// Combine the strings.
	let mut out: Vec<String> = Vec::new();

	// Return a shortened version.
	if true == short {
		if d > 0 {
			out.push(format!("{:02}", d));
		}

		// Always do hours, minutes, and seconds.
		out.push(format!("{:02}", h));
		out.push(format!("{:02}", m));
		out.push(format!("{:02}", s));

		return out.join(":");
	}

	// A longer version.
	if d > 0 {
		out.push(format!(
			"{} {}",
			d,
			inflect(d, "day".to_string(), "days".to_string()),
		));
	}
	if h > 0 {
		out.push(format!(
			"{} {}",
			h,
			inflect(h, "hour".to_string(), "hours".to_string()),
		));
	}
	if m > 0 {
		out.push(format!(
			"{} {}",
			m,
			inflect(m, "minute".to_string(), "minutes".to_string()),
		));
	}
	if s > 0 {
		out.push(format!(
			"{} {}",
			s,
			inflect(s, "second".to_string(), "seconds".to_string()),
		));
	}

	match out.len() {
		1 => out.pop().unwrap_or("0 seconds".to_string()),
		2 => out.join(" and "),
		_ => {
			let last = out.pop().unwrap_or("".to_string());
			format!("{}, and {}", out.join(", "), last)
		},
	}
}



// ---------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------

/// Print an error and exit.
fn error(text: Box<Error>) {
	eprintln!(
		"{} {}",
		ansi_term::Colour::Red.bold().paint("Error:"),
		text,
	);
	std::process::exit(1);
}

/// Print a warning, but do not exit.
fn warning(text: Box<Error>) {
	eprintln!(
		"{} {}",
		ansi_term::Colour::Yellow.bold().paint("Warning:"),
		text,
	);
}

/// Print a notice. This is generally only used when --debug is set.
fn notice(text: String) {
	println!(
		"{} {}",
		ansi_term::Colour::Purple.bold().paint("Notice:"),
		text,
	);
}

/// Log the cumulative compression results for an image to a user-
/// specified location.
fn log<P: AsRef<Path>>(log_path: P, image_path: P, saved: u64, elapsed: u64) -> Result<()> {
	let end_size = get_file_size(image_path.as_ref().to_path_buf())?;
	let start_size = end_size + saved;
	let image = get_file_canonical(image_path.as_ref().to_path_buf())?;

	// The log is set as a directory; we want a file instead.
	let log = format!("{}/flaca.log", get_file_canonical(log_path.as_ref().to_path_buf()).unwrap_or(".".to_string()));

	// Put together a human-readable status string.
	let status: String = match saved {
		0 => "No change.".to_string(),
		_ => format!(
			"Saved {} bytes in {} seconds.",
			saved,
			elapsed,
		),
	};

	// Open/create the log file.
	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open(PathBuf::from(log).to_path_buf())?;

	// Append the line.
	if let Err(_) = writeln!(
		file,
		"{} \"{}\" {} {} {}",
		get_local_now().to_rfc3339(),
		image,
		start_size,
		end_size,
		status,
	) {
		return Err(Error::new(ErrorKind::Other, "Unable to log results.").into());
	}

	Ok(())
}

/// Return a singular or plural version of a string given a count.
fn inflect(count: u64, singular: String, plural: String) -> String {
	match count {
		1 => singular,
		_ => plural,
	}
}

/// Pad a string on the left to ensure a minimum overall length.
fn pad_left(text: String, pad_length: u64, pad_string: u8) -> String {
	let text_length: u64 = text.len() as u64;
	if pad_length <= 0 || pad_length <= text_length {
		return text;
	}

	format!(
		"{}{}",
		String::from_utf8(vec![pad_string; (pad_length - text_length) as usize]).unwrap_or("".to_string()),
		text,
	)
}

/// Pad a string on the right to ensure a minimum overall length.
fn pad_right(text: String, pad_length: u64, pad_string: u8) -> String {
	let text_length: u64 = text.len() as u64;
	if pad_length <= 0 || pad_length <= text_length {
		return text;
	}

	format!(
		"{}{}",
		text,
		String::from_utf8(vec![pad_string; (pad_length - text_length) as usize]).unwrap_or("".to_string()),
	)
}

#[derive(Debug)]
/// A very simple progress bar.
struct Progress {
	tick: u64,
	total: u64,
	start: SystemTime,
	msg: Option<String>,
	show: bool,
	show_bar: bool,
	show_elapsed: bool,
	show_percent: bool,
	show_progress: bool,
	last: String,
}

impl Default for Progress {
	/// Default configurations for a progress bar.
	fn default() -> Progress {
		Progress {
			tick: 0,
			total: 0,
			start: SystemTime::now(),
			msg: None,
			show: true,
			show_bar: true,
			show_elapsed: true,
			show_percent: true,
			show_progress: true,
			last: "".to_string(),
		}
	}
}

impl Progress {
	/// Start a new progress bar.
	fn new(total: u64, show: bool) -> Progress {
		Progress {
			total: total,
			show: show,
			..Progress::default()
		}
	}

	/// Set the current tick.
	fn set_tick(&mut self, mut tick: u64) {
		if tick > self.total {
			tick = self.total;
		}

		if self.tick != tick {
			self.tick = tick;
			self.redraw(false);
		}
	}

	/// Set or unset a message to append to the end of the line.
	fn set_msg(&mut self, msg: Option<String>) {
		if msg != self.msg {
			self.msg = msg;
			self.redraw(false);
		}
	}

	/// Finish a progress bar.
	fn finish(&mut self) {
		self.tick = self.total;

		if true == self.show {
			println!("{}", self.get_line());
		}
	}

	/// (Re)draw a progress bar.
	///
	/// This triggers automatically any time the contents of the line
	/// change, but also has to be redone when lines are shifted
	/// before it, etc.
	fn redraw(&mut self, force: bool) {
		// Nothing to redraw if we're hiding the bar.
		if false == self.show {
			return;
		}

		let line: String = self.get_line();

		// We have new line content.
		if line != self.last {
			self.last = line.to_string();
		}
		// Nothing's changed and we aren't forcing action, so let's
		// bail.
		else if false == force {
			return;
		}

		// Print it!
		eprint!("{}\r", line);
	}

	/// Insert a line before the progress bar.
	///
	/// A special handler is required as any new line sent to STDOUT
	/// will cover the previous incarnation of the progress bar. We
	/// need to add a new one.
	fn prepend(&mut self, text: String) {
		println!("{}", self.fill_row(text));
		self.redraw(true);
	}

	/// Build a line given the progress bar's settings and data.
	fn get_line(&mut self) -> String {
		if false == self.show {
			return "".to_string();
		}

		let mut out: Vec<String> = Vec::new();

		if true == self.show_elapsed {
			out.push(format!("[{}]", self.get_elapsed()));
		}

		if true == self.show_bar {
			out.push(self.get_bar());
		}

		if true == self.show_progress {
			out.push(self.get_progress());
		}

		if true == self.show_percent {
			out.push(self.get_percent());
		}

		if let Some(msg) = self.msg.to_owned() {
			out.push(msg);
		}

		format!("{}", self.fill_row(out.join(" ")))
	}

	/// Fill a row with whitespace so that it stretches the width of
	/// the terminal.
	fn fill_row(&mut self, text: String) -> String {
		if let Some((w, _)) = term_size::dimensions() {
			return pad_right(text, w as u64, b' ');
		}

		return text;
	}

	/// Get the ### part of the progress bar.
	fn get_bar(&self) -> String {
		// Figure out the bar widths.
		let width: u64 = 40;
		let width1: u64 = match self.total {
			0 => 0,
			_ => (self.tick as f64 / self.total as f64 * width as f64).floor() as u64,
		};
		let width2: u64 = width - width1;

		// Draw up the bar strings.
		let bar1: String = match width1 {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x as usize]).unwrap_or("".to_string()),
		};
		let bar2: String = match width2 {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x as usize]).unwrap_or("".to_string()),
		};

		// Return a right-looking bar!
		format!(
			"{}{}",
			ansi_term::Colour::Cyan.bold().paint(bar1),
			ansi_term::Colour::Blue.paint(bar2),
		)
	}

	/// Get the elapsed time for the progress bar.
	fn get_elapsed(&self) -> String {
		let now = SystemTime::now();
		if let Ok(y) = now.duration_since(self.start) {
			return get_nice_time(y.as_secs(), true);
		}

		"00:00:00".to_string()
	}

	/// Get the percent done for the progress bar.
	fn get_percent(&self) -> String {
		if 0 == self.total {
			return "  0%".to_string();
		}

		format!("{:>3.*}%", 0, self.tick as f64 / self.total as f64 * 100 as f64)
	}

	/// Get the done/total for the progress bar.
	fn get_progress(&self) -> String {
		let len: u64 = format!("{}", self.total).len() as u64;
		let done: String = pad_left(format!("{}", self.tick), len, b' ');

		format!(
			"{}/{}",
			ansi_term::Colour::Cyan.bold().paint(done),
			ansi_term::Colour::Blue.paint(format!("{}", self.total)),
		)
	}
}



// ---------------------------------------------------------------------
// Binary
// ---------------------------------------------------------------------

fn main() {
	// Options.
	let mut opts: Options = Options::from_env();
	header(&mut opts);

	let start = SystemTime::now();
	let images = opts.raw.to_owned();
	let mut num: u64 = 0;
	let mut saved: u64 = 0;
	let mut progress = Progress::new(images.len() as u64, false == opts.quiet);

	// Available JPEG encoders.
	let jpeg_encoders: Vec<(Encoder, PathBuf)> = {
		let mut tmp: Vec<(Encoder, PathBuf)> = Vec::new();

		if let Ok(x) = opts.bin_mozjpeg {
			tmp.push(x);
		}

		if let Ok(x) = opts.bin_jpegoptim {
			tmp.push(x);
		}

		tmp
	};

	// Available PNG encoders.
	let png_encoders: Vec<(Encoder, PathBuf)> = {
		let mut tmp: Vec<(Encoder, PathBuf)> = Vec::new();

		if let Ok(x) = opts.bin_pngout {
			tmp.push(x);
		}

		if let Ok(x) = opts.bin_oxipng {
			tmp.push(x);
		}

		if let Ok(x) = opts.bin_zopflipng {
			tmp.push(x);
		}

		tmp
	};

	// Loop the images.
	for i in &images {
		num = num + 1;

		// Copy encoders, if any.
		let encoders: Vec<(Encoder, PathBuf)> = match get_image_type(i.path.to_path_buf()) {
			Ok(ImageType::Jpg) => jpeg_encoders.to_owned(),
			Ok(ImageType::Png) => png_encoders.to_owned(),
			_ => Vec::new(),
		};
		if encoders.len() < 1 {
			continue;
		}

		// Print the current image path so we know what's going on.
		if false == opts.quiet {
			progress.prepend(format!(
				"{} {}",
				ansi_term::Colour::Purple.bold().paint(get_local_now().to_rfc3339()),
				get_file_canonical(i.path.to_path_buf()).unwrap_or("MISSING".to_string())
			));
		}

		let mut i_saved: u64 = 0;
		let mut i_elapsed: u64 = 0;

		// Loop the encoders.
		for (encoder, bin) in encoders {
			// Make sure the image still exists.
			if ! i.exists() {
				progress.prepend(format!(
					"{} {}",
					ansi_term::Colour::Yellow.bold().paint("Warning:"),
					"The image has gone missing.",
				));

				break;
			}

			if true == opts.debug {
				progress.prepend(format!(
					"    ↳ Running {} from {}.",
					ansi_term::Colour::Purple.paint(encoder.get_nice_name()),
					get_file_canonical(bin.to_path_buf()).unwrap_or("MISSING".to_string())
				));
			}

			if let Ok(result) = i.compress(encoder, bin, false == opts.pretend) {
				// Saved something?
				if let Ok(saved_inner) = result.get_saved() {
					saved += saved_inner;
					i_saved += saved_inner;

					progress.set_msg(Some(format!(
						"{} {}",
						ansi_term::Colour::Green.bold().paint("Saved:"),
						ansi_term::Style::new().bold().paint(get_nice_size(saved)),
					)));

					i_elapsed += result.get_elapsed().unwrap_or(0);
				}
			}
		}

		// Log the combined results for this image.
		if opts.log.is_some() {
			if let Err(_) = log(opts.log.to_owned().unwrap(), i.path.to_path_buf(), i_saved, i_elapsed) {
				progress.prepend(format!(
					"{} {}",
					ansi_term::Colour::Yellow.bold().paint("Warning:"),
					"Unable to log results.",
				));
			}
		}

		progress.set_tick(num);
	}

	progress.finish();

	// Print a summary maybe.
	if false == opts.quiet {
		println!("");

		let end = SystemTime::now();
		let mut elapsed: u64 = 0;
		if let Ok(x) = end.duration_since(start) {
			elapsed = x.as_secs();
		}

		// General summary.
		println!(
			"{} {} {}.\n{} {}.",
			ansi_term::Colour::Cyan.bold().paint("Checked:"),
			num,
			inflect(num, "image".to_string(), "images".to_string()),
			ansi_term::Colour::Cyan.bold().paint("Elapsed:"),
			get_nice_time(elapsed, false),
		);

		if saved > 0 {
			if true == opts.pretend {
				notice("Re-run Flaca without '--pretend' to see real change.".to_string());
			}
			else {
				println!(
					"{} {}",
					ansi_term::Colour::Green.bold().paint("Saved:  "),
					get_nice_size(saved),
				);
			}
		}
		else {
			println!(
				"{} 0B",
				ansi_term::Colour::Yellow.bold().paint("Saved:  "),
			);
		}
	}
}

/// A fun little CLI introductory header.
fn header(opts: &mut Options) {
	// Don't print if we're supposed to be quiet.
	if opts.quiet {
		return;
	}

	println!(
"
             ,--._,--.
           ,'  ,'   ,-`.
(`-.__    /  ,'   /
 `.   `--'        \\__,--'-.
   `--/       ,-.  ______/
     (o-.     ,o- /
      `. ;        \\
       |:          \\    {} {}
      ,'`       ,   \\
     (o o ,  --'     :  {} {}
      \\--','.        ;  {}  {}
       `;;  :       /
        ;'  ;  ,' ,'    {}
        ,','  :  '
        \\ \\   :
         `

",
		ansi_term::Colour::Purple.bold().paint("Flaca"),
		ansi_term::Style::new().bold().paint(format!("v{}", VERSION)),
		ansi_term::Colour::Blue.bold().paint("Images:"),
		opts.total_images(),
		ansi_term::Colour::Blue.bold().paint("Space:"),
		get_nice_size(opts.total_image_size()),
		"Ready, Set, Goat!",
	);
}
