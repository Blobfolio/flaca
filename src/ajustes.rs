/*!
Flaca: Settings

*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

use ansi_term::{Style, Colour};
use crate::imagen::{Imagen, ImagenKind};
use mando::lugar::Lugar;
use mando::pantalla::Pantalla;
use rayon::prelude::*;
use std::error::Error;
use std::path::PathBuf;
use std::time::SystemTime;



#[derive(Debug, Clone)]
pub struct Ajustes {
	display: Pantalla,
	min_age: u64,
	max_age: u64,
	min_size: u64,
	max_size: u64,
	skip: ImagenKind,
	jpegoptim: Imagen,
	mozjpeg: Imagen,
	oxipng: Imagen,
	pngout: Imagen,
	zopflipng: Imagen,
	paths: Vec<Lugar>,
}

impl Default for Ajustes {
	fn default() -> Ajustes {
		Ajustes {
			display: Pantalla::new(),
			min_age: 0,
			max_age: 0,
			min_size: 0,
			max_size: 0,
			skip: ImagenKind::None,
			jpegoptim: Imagen::Jpegoptim(Lugar::None),
			mozjpeg: Imagen::MozJPEG(Lugar::None),
			oxipng: Imagen::Oxipng(Lugar::None),
			pngout: Imagen::Pngout(Lugar::None),
			zopflipng: Imagen::Zopflipng(Lugar::None),
			paths: vec![],
		}
	}
}

impl Ajustes {
	pub fn init() -> Ajustes {
		let args = clap::App::new("Flaca")
			.version(env!("CARGO_PKG_VERSION"))
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
				.validator(Ajustes::validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("max_age")
				.long("max-age")
				.alias("max_age")
				.help("Ignore files older than this.")
				.takes_value(true)
				.validator(Ajustes::validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("min_size")
				.long("min-size")
				.alias("min_size")
				.help("Ignore files smaller than this.")
				.takes_value(true)
				.validator(Ajustes::validate_min_max)
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("max_size")
				.long("max-size")
				.alias("max_size")
				.help("Ignore files larger than this.")
				.takes_value(true)
				.validator(Ajustes::validate_min_max)
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
			.get_matches();

		// Figure out display first.
		let display: Pantalla =
			if args.is_present("debug") {
				Pantalla::new_debug()
			}
			else if args.is_present("quiet") {
				Pantalla::new_quiet()
			}
			else {
				Pantalla::new()
			};

		// Now skippingness.
		let mut skip: ImagenKind = match args.value_of("skip") {
			Some(x) => match x {
				"jpg" => ImagenKind::Jpg,
				"jpeg" => ImagenKind::Jpg,
				"png" => ImagenKind::Png,
				_ => ImagenKind::None,
			},
			None => ImagenKind::None,
		};

		// Find JPEG encoders for real.
		let mut jpegoptim = Imagen::Jpegoptim(Lugar::None);
		let mut mozjpeg = Imagen::MozJPEG(Lugar::None);
		if skip != ImagenKind::Jpg {
			if let Some(x) = args.value_of("x_jpegoptim") {
				jpegoptim = Imagen::Jpegoptim(Lugar::new(x));
			}
			if let Some(x) = args.value_of("x_mozjpeg") {
				mozjpeg = Imagen::MozJPEG(Lugar::new(x));
			}
		}

		// And now the PNG encoders.
		let mut oxipng = Imagen::Oxipng(Lugar::None);
		let mut pngout = Imagen::Pngout(Lugar::None);
		let mut zopflipng = Imagen::Zopflipng(Lugar::None);
		if skip != ImagenKind::Png {
			if let Some(x) = args.value_of("x_oxipng") {
				oxipng = Imagen::Oxipng(Lugar::new(x));
			}
			if let Some(x) = args.value_of("x_pngout") {
				pngout = Imagen::Pngout(Lugar::new(x));
			}
			if let Some(x) = args.value_of("x_zopflipng") {
				zopflipng = Imagen::Zopflipng(Lugar::new(x));
			}
		}

		// No JPEG encoders found?
		if
			skip != ImagenKind::Jpg &&
			! jpegoptim.is_some() &&
			! mozjpeg.is_some()
		{
			if skip == ImagenKind::None {
				skip = ImagenKind::Jpg;
				display.warning("No JPEG encoders were found.");
			}
			else if skip == ImagenKind::Png {
				// If listing, just exit.
				if args.is_present("list_only") {
					std::process::exit(0);
				}

				// Otherwise print an error and exit.
				display.error("No JPEG encoders were found.");
			}
		}

		// No PNG encoders found?
		if
			skip != ImagenKind::Png &&
			! oxipng.is_some() &&
			! pngout.is_some() &&
			! zopflipng.is_some()
		{
			if skip == ImagenKind::None {
				skip = ImagenKind::Png;
				display.warning("No PNG encoders were found.");
			}
			else if skip == ImagenKind::Jpg {
				// If listing, just exit.
				if args.is_present("list_only") {
					std::process::exit(0);
				}

				display.error("No PNG encoders were found.");
			}
		}

		// Find images!
		let raw: Vec<Lugar> = args
			.values_of("INPUT")
			.unwrap()
			.map(|x| Lugar::new(x))
			.collect();

		let images: Vec<Lugar> = match skip {
			ImagenKind::Jpg => Lugar::walk(&raw, false, true, Some(Ajustes::walk_png)),
			ImagenKind::Png => Lugar::walk(&raw, false, true, Some(Ajustes::walk_jpg)),
			ImagenKind::None => Lugar::walk(&raw, false, true, Some(Ajustes::walk_img)),
		};

		// Our settings!
		let tmp = Ajustes {
			display: display,
			min_age: Ajustes::parse_min_max_age(args.value_of("min_age")),
			max_age: Ajustes::parse_min_max_age(args.value_of("max_age")),
			min_size: Ajustes::parse_min_max_size(args.value_of("min_size")),
			max_size: Ajustes::parse_min_max_size(args.value_of("max_size")),
			skip: skip,
			jpegoptim: jpegoptim,
			mozjpeg: mozjpeg,
			oxipng: oxipng,
			pngout: pngout,
			zopflipng: zopflipng,
			paths: images,
		};

		// If we are just listing, let's list and die.
		if args.is_present("list_only") {
			for v in &tmp.paths {
				if let Ok(x) = v.path() {
					tmp.display.plain(x);
				}
			}

			std::process::exit(0);
		}

		tmp.print_header();

		// No images, nothing to do.
		if 0 == tmp.paths.len() {
			tmp.display.error("No qualifying images were found.");
		}

		tmp
	}

	pub fn compress(&self) -> Result<(u64, u64), Box<dyn Error>> {
		// Start your engines.
		let mut saved: u64 = 0;
		let start_time = SystemTime::now();

		let jpg: Vec<Lugar> =
			if self.skip != ImagenKind::Jpg {
				// TODO
			}
			else {
				vec![]
			};

		// Wrap it up.
		let elapsed: u64 = Lugar::time_diff(SystemTime::now(), start_time)?;
		Ok((saved, elapsed))
	}

	fn validate_min_max(val: String) -> Result<(), String> {
		if let Ok(x) = val.parse::<u64>() {
			if x > 0 {
				return Ok(());
			}
		}

		Err("Value must be greater than zero.".to_string())
	}

	fn parse_min_max_age(val: Option<&str>) -> u64 {
		match val
			.unwrap_or("0")
			.parse::<u64>() {
				Ok(y) => y * 60,
				_ => 0,
			}
	}

	fn parse_min_max_size(val: Option<&str>) -> u64 {
		match val
			.unwrap_or("0")
			.parse::<u64>() {
				Ok(y) => y,
				_ => 0,
			}
	}

	fn walk_img(path: &walkdir::DirEntry) -> bool {
		if path.path().is_file() {
			path.file_name()
				.to_str()
				.map(|s| {
					let lower = s.to_lowercase();

					lower.ends_with(".png") ||
					lower.ends_with(".jpg") ||
					lower.ends_with(".jpeg")
				})
				.unwrap_or(false)
		}
		else {
			true
		}
	}

	fn walk_jpg(path: &walkdir::DirEntry) -> bool {
		if path.path().is_file() {
			path.file_name()
				.to_str()
				.map(|s| {
					let lower = s.to_lowercase();

					lower.ends_with(".jpg") ||
					lower.ends_with(".jpeg")
				})
				.unwrap_or(false)
		}
		else {
			true
		}
	}

	fn walk_png(path: &walkdir::DirEntry) -> bool {
		if path.path().is_file() {
			path.file_name()
				.to_str()
				.map(|s| s.to_lowercase().ends_with(".png"))
				.unwrap_or(false)
		}
		else {
			true
		}
	}

	fn print_header(&self) {
		// Try to be quiet.
		if self.display.show_quiet() {
			return;
		}

		let count: u64 = self.paths.len() as u64;
		let size: u64 = Lugar::du(&self.paths);

		self.display.plain(format!("
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
			Colour::Purple.bold().paint("Flaca"),
			Style::new().bold().paint(format!("v{}", env!("CARGO_PKG_VERSION"))),
			Colour::Blue.bold().paint("Images:"),
			count,
			Colour::Blue.bold().paint("Space:"),
			Pantalla::nice_size(size),
			"Ready, Set, Goat!",
		));
	}
}
