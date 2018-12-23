// Flaca: Mundo
//
// The runtime environment.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use crate::VERSION;
use crate::lugar::Lugar;
use crate::diario::{Diario, Nivel};
use crate::granjero::{Granjero, Tipo};
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
/// Runtime settings.
pub struct Mundo {
	pub min_age: Option<u64>,
	pub max_age: Option<u64>,
	pub min_size: Option<u64>,
	pub max_size: Option<u64>,
	pub skip: Option<Tipo>,
	pub jpg: Vec<Granjero>,
	pub png: Vec<Granjero>,
	pub input: Vec<Lugar>,
	pub output: Diario,
}

impl Default for Mundo {
	fn default() -> Mundo {
		Mundo {
			min_age: None,
			max_age: None,
			min_size: None,
			max_size: None,
			skip: None,
			jpg: Mundo::jpg_encoders(None, None),
			png: Mundo::png_encoders(None, None, None),
			input: Vec::new(),
			output: Diario::default(),
		}
	}
}

impl Mundo {
	/// Parse runtime settings from CLI.
	pub fn start() -> Mundo {
		Mundo::from_env(&clap::App::new("Flaca")
			.version(VERSION)
			.author("Blobfolio, LLC <hello@blobfolio.com>")
			.about("Losslessly compress la mierda out of JPEG and PNG images.")
			.arg(clap::Arg::with_name("debug")
				.short("d")
				.long("debug")
				.alias("verbose")
				.conflicts_with("quiet")
				.help("Print verbose information to STDOUT.")
			)
			.arg(clap::Arg::with_name("list_only")
				.long("list-only")
				.alias("list")
				.alias("list_only")
				.conflicts_with("quiet")
				.help("Print a list of qualifying images and exit.")
			)
			.arg(clap::Arg::with_name("quiet")
				.short("q")
				.long("quiet")
				.conflicts_with("debug")
				.conflicts_with("list_only")
				.help("Suppress STDOUT. This has no effect on errors.")
			)
			.arg(clap::Arg::with_name("log")
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
				.long("min-age")
				.alias("min_age")
				.help("Ignore files younger than this.")
				.takes_value(true)
				.validator(Mundo::validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("max_age")
				.long("max-age")
				.alias("max_age")
				.help("Ignore files older than this.")
				.takes_value(true)
				.validator(Mundo::validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("min_size")
				.long("min-size")
				.alias("min_size")
				.help("Ignore files smaller than this.")
				.takes_value(true)
				.validator(Mundo::validate_min_max)
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("max_size")
				.long("max-size")
				.alias("max_size")
				.help("Ignore files larger than this.")
				.takes_value(true)
				.validator(Mundo::validate_min_max)
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("skip")
				.short("s")
				.long("skip")
				.help("Skip images of this type.")
				.possible_values(&["jpeg", "jpg", "png"])
				.takes_value(true)
				.value_name("FORMAT")
			)
			.arg(clap::Arg::with_name("x_jpegoptim")
				.long("x-jpegoptim")
				.alias("jpegoptim")
				.alias("x_jpegoptim")
				.help("Alternate binary path for Jpegoptim.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("x_mozjpeg")
				.long("x-mozjpeg")
				.alias("jpegtran")
				.alias("mozjpeg")
				.alias("x-jpegtran")
				.alias("x_jpegtran")
				.alias("x_mozjpeg")
				.help("Alternate binary path for MozJPEG.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("x_oxipng")
				.long("x-oxipng")
				.alias("oxipng")
				.alias("x_oxipng")
				.help("Alternate binary path for Oxipng.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("x_pngout")
				.long("x-pngout")
				.alias("pngout")
				.alias("x_pngout")
				.help("Alternate binary path for Pngout.")
				.takes_value(true)
				.value_name("BIN")
			)
			.arg(clap::Arg::with_name("x_zopflipng")
				.long("x-zopflipng")
				.alias("x_zopflipng")
				.alias("zopflipng")
				.help("Alternate binary path for Zopflipng.")
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
			.get_matches()
		)
	}

	/// Internal helper to parse arguments from CLI.
	fn from_env(args: &clap::ArgMatches) -> Mundo {
		// Display verbosity is derived from separate flags.
		let verbosity: Nivel =
			// Fill the screen with information.
			if args.is_present("debug") {
				Nivel::Debug
			}
			// Don't print anything to STDOUT.
			else if args.is_present("quiet") {
				Nivel::Quiet
			}
			// Print a list of found files and exit.
			else if args.is_present("list_only") {
				Nivel::List
			}
			// Show a progress bar.
			else {
				Nivel::Standard
			};

		// Log path should be a directory if provided.
		let log: Option<Lugar> = match args.value_of("log") {
			Some(x) => {
				let tmp = Lugar::Path(x.into());
				if tmp.is_dir() {
					Some(tmp)
				} else {
					None
				}
			},
			None => None,
		};

		// Figure out skip early to save some overhead later.
		let skip: Option<Tipo> = match args.value_of("skip") {
			Some(x) => match x {
				"jpg" => Some(Tipo::Jpg),
				"jpeg" => Some(Tipo::Jpg),
				"png" => Some(Tipo::Png),
				_ => None,
			},
			None => None,
		};

		// We have enough to get going, but still need to collect
		// images.
		let mut out: Mundo = Mundo {
			min_age: Mundo::parse_min_max(args.value_of("min_age")),
			max_age: Mundo::parse_min_max(args.value_of("max_age")),
			min_size: Mundo::parse_min_max(args.value_of("min_size")),
			max_size: Mundo::parse_min_max(args.value_of("max_size")),
			skip: skip,
			jpg: match skip {
				Some(Tipo::Jpg) => {
					Vec::new()
				},
				_ => {
					Mundo::jpg_encoders(
						args.value_of("x_jpegoptim").map(|x| Lugar::Path(x.into())),
						args.value_of("x_mozjpeg").map(|x| Lugar::Path(x.into())),
					)
				},
			},
			png: match skip {
				Some(Tipo::Png) => {
					Vec::new()
				},
				_ => {
					Mundo::png_encoders(
						args.value_of("x_oxipng").map(|x| Lugar::Path(x.into())),
						args.value_of("x_pngout").map(|x| Lugar::Path(x.into())),
						args.value_of("x_zopflipng").map(|x| Lugar::Path(x.into())),
					)
				},
			},
			input: Vec::new(),
			output: Diario::new(verbosity, log),
		};

		// Now we need to see if any images map.
		let mut images = out.parse_images(
			args
				.values_of("INPUT")
				.unwrap()
				.map(|x| Lugar::Path(x.into()))
				.collect(),
		);

		// Clean up the images a touch for saving, if needed.
		if images.len() > 0 {
			images.sort();
			images.dedup();

			out.input = images
				.iter()
				.map(|x| Lugar::Path(x.into()))
				.collect();
		}

		out
	}

	/// Build a list of all possible JPEG encoders, factoring user-
	/// specified paths.
	fn jpg_encoders(
		jpegoptim: Option<Lugar>,
		mozjpeg: Option<Lugar>,
	) -> Vec<Granjero> {
		// Available JPEG apps.
		let mut jpg: Vec<Granjero> = Vec::new();

		if let Ok(x) = Granjero::Mozjpeg(mozjpeg).cmd_path() {
			jpg.push(Granjero::Mozjpeg(Some(Lugar::Path(x.into()))));
		}

		if let Ok(x) = Granjero::Jpegoptim(jpegoptim).cmd_path() {
			jpg.push(Granjero::Jpegoptim(Some(Lugar::Path(x.into()))));
		}

		jpg
	}

	/// Build a list of all possible PNG encoders, factoring user-
	/// specified paths.
	fn png_encoders(
		oxipng: Option<Lugar>,
		pngout: Option<Lugar>,
		zopflipng: Option<Lugar>,
	) -> Vec<Granjero> {
		// Available PNG apps.
		let mut png: Vec<Granjero> = Vec::new();

		if let Ok(x) = Granjero::Pngout(pngout).cmd_path() {
			png.push(Granjero::Pngout(Some(Lugar::Path(x.into()))));
		}

		if let Ok(x) = Granjero::Oxipng(oxipng).cmd_path() {
			png.push(Granjero::Oxipng(Some(Lugar::Path(x.into()))));
		}

		if let Ok(x) = Granjero::Zopflipng(zopflipng).cmd_path() {
			png.push(Granjero::Zopflipng(Some(Lugar::Path(x.into()))));
		}

		png
	}

	/// Recursively find all applicable image files given the paths
	/// passed through CLI.
	///
	/// Results are returned as canonical Strings for easy sorting and
	/// deduplication.
	fn parse_images(&self, files: Vec<Lugar>) -> Vec<String> {
		let mut out = Vec::new();

		for file in files {
			// Recurse directories.
			if file.is_dir() {
				let files = file.path()
					.unwrap()
					.read_dir()
					.unwrap()
					.map(|x| Lugar::Path(x.unwrap().path().to_owned()))
					.collect();
				out.extend(self.parse_images(files));
			}
			// Just a regular old file.
			else if file.is_file() {
				// Should be an expandable path.
				if let Ok(path) = file.canonical() {
					// Check extension first.
					if let Ok(ext) = Tipo::from(file.to_owned()) {
						// Skipping this type.
						if self.skip == Some(ext) {
							continue;
						}

						// Check file size.
						if self.min_size.is_some() || self.max_size.is_some() {
							if let Ok(size) = file.size() {
								if (self.min_size.is_some() && size < self.min_size.unwrap()) || (self.max_size.is_some() && size > self.max_size.unwrap()) {
									continue;
								}
							} else {
								continue;
							}
						}

						// Check file time.
						if self.min_age.is_some() || self.max_age.is_some() {
							if let Ok(age) = file.mtime_from_now() {
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

	/// Internal callback to validate min/max age/size args.
	///
	/// If supplied, the value must be greater than nothing.
	fn validate_min_max(val: String) -> Result<(), String> {
		if let Ok(x) = val.parse::<u64>() {
			if x > 0 {
				return Ok(());
			}
		}

		Err("Value must be greater than zero.".to_string())
	}

	/// Internal callback to cast min/max age/size args.
	///
	/// If supplied, the value must be greater than nothing.
	fn parse_min_max(val: Option<&str>) -> Option<u64> {
		match val
			.unwrap_or("0")
			.parse::<u64>() {
			Ok(0) => None,
			Ok(y) => Some(y * 60),
			_ => None,
		}
	}

	/// The total number of images found.
	pub fn total_images(&self) -> u64 {
		return self.input.len() as u64;
	}

	/// The total disk size taken up by found images.
	pub fn total_size(&self) -> u64 {
		let mut size: u64 = 0;

		for ref i in &self.input {
			if let Ok(x) = i.size() {
				size += x;
			}
		}

		size
	}
}
