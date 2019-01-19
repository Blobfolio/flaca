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
use chrono::Timelike;
use crate::imagen::{Cosecha, Imagen, ImagenKind};
use mando::lugar::Lugar;
use mando::pantalla::Pantalla;
use rayon::prelude::*;
use std::error::Error;
use std::path::PathBuf;
use std::time::SystemTime;
use std::sync::Mutex;



#[derive(Debug, Clone)]
pub struct Ajustes {
	min_age: u64,
	max_age: u64,
	min_size: u64,
	max_size: u64,
	skip: ImagenKind,
	log: Lugar,
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
			min_age: 0,
			max_age: 0,
			min_size: 0,
			max_size: 0,
			skip: ImagenKind::None,
			log: Lugar::None,
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
	pub fn init() {
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
				.validator(validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("max_age")
				.long("max-age")
				.alias("max_age")
				.help("Ignore files older than this.")
				.takes_value(true)
				.validator(validate_min_max)
				.value_name("MINUTES")
			)
			.arg(clap::Arg::with_name("min_size")
				.long("min-size")
				.alias("min_size")
				.help("Ignore files smaller than this.")
				.takes_value(true)
				.validator(validate_min_max)
				.value_name("BYTES")
			)
			.arg(clap::Arg::with_name("max_size")
				.long("max-size")
				.alias("max_size")
				.help("Ignore files larger than this.")
				.takes_value(true)
				.validator(validate_min_max)
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

		// Configure multi-threading.
		let num_cpus = num_cpus::get();
        let thread_count = num_cpus + (num_cpus >> 1);
		let _ = rayon::ThreadPoolBuilder::new()
			.num_threads(thread_count)
			.build_global();

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

		// Log path?
		let log: Lugar = match args.value_of("log") {
			Some(x) => {
				let mut tmp: Lugar = Lugar::new(x);
				if let Err(_) = tmp.push("flaca.log") {
					tmp = Lugar::None;
				}
				tmp
			},
			_ => Lugar::None,
		};

		// Size and age.
		let min_age: u64 = parse_min_max_age(args.value_of("min_age"));
		let max_age: u64 = parse_min_max_age(args.value_of("max_age"));
		let min_size: u64 = parse_min_max_size(args.value_of("min_size"));
		let max_size: u64 = parse_min_max_size(args.value_of("max_size"));

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
		}
			.into_par_iter()
			.filter(|x| {
				// Filter size.
				if max_size > 0 || min_size > 0 {
					let size = x.size().unwrap_or(0);
					if
						0 == size ||
						(max_size > 0 && size > max_size) ||
						(min_size > 0 && size < min_size)
					{
						return false;
					}
				}

				// Filter age.
				if max_age > 0 || min_age > 0 {
					let age = x.age().unwrap_or(0);
					if
						0 == age ||
						(max_age > 0 && age > max_age) ||
						(min_age > 0 && age < min_age)
					{
						return false;
					}
				}

				true
			})
			.collect();

		// Our settings!
		let out = Ajustes {
			min_age: min_age,
			max_age: max_age,
			min_size: min_size,
			max_size: max_size,
			skip: skip,
			log: log,
			jpegoptim: jpegoptim,
			mozjpeg: mozjpeg,
			oxipng: oxipng,
			pngout: pngout,
			zopflipng: zopflipng,
			paths: images,
		};

		// If we are just listing, let's list and die.
		if args.is_present("list_only") {
			for v in &out.paths {
				if let Ok(x) = v.path() {
					display.plain(x);
				}
			}

			std::process::exit(0);
		}

		out.print_header(&display);

		// No images, nothing to do.
		if 0 == out.paths.len() {
			display.error("No qualifying images were found.");
		}

		// Debug information?
		if args.is_present("debug") {
			display.notice("Debug mode enabled.");

			display.notice(format!("Using {} threads.", thread_count));

			// Age restrictions.
			if min_age > 0 {
				display.notice(format!("Minimum file age: {} seconds.", min_age));
			}

			if max_age > 0 {
				display.notice(format!("Maximum file age: {} seconds.", max_age));
			}

			// Size restrictions.
			if min_size > 0 {
				display.notice(format!("Minimum file size: {} bytes.", min_size));
			}

			if max_size > 0 {
				display.notice(format!("Maximum file size: {} bytes.", max_size));
			}

			// Are we skipping?
			if skip == ImagenKind::Jpg {
				display.notice("Skipping JPEG images.");
			}
			else if skip == ImagenKind::Png {
				display.notice("Skipping PNG images.");
			}

			// Are we logging?
			if out.log.is_some() {
				display.notice(format!("Logging results to {}.", out.log));
			}

			// Available encoders.
			for v in &[&out.jpegoptim, &out.mozjpeg, &out.oxipng, &out.pngout, &out.zopflipng] {
				let bin = v.bin_path();
				if bin.is_none() {
					display.warning(format!("{} is not installed.", v));
				}
				else {
					display.notice(format!("Found {} at {}", v, bin));
				}
			}

			display.plain("");
		}

		// Start compressing!
		let shared_display = Mutex::new(display);
		if let Ok((saved, elapsed)) = out.compress(&shared_display) {
			// Talk about what went right.
			let display = shared_display.lock().unwrap();

			display.plain("");
			display.success(format!("Finished in {}", Pantalla::nice_time(elapsed, false)));
			if saved > 0 {
				display.success(format!("Saved {}", Pantalla::nice_size(saved)));
			}
			else {
				display.warning("No lossless savings were possible.");
			}
		}
		// Compression failed.
		else {
			let display = shared_display.lock().unwrap();
			display.error("Compression failed.");
		}
	}

	pub fn compress(&self, shared_display: &Mutex<Pantalla>) -> Result<(u64, u64), Box<dyn Error>> {
		// Start your engines.
		let start_time = SystemTime::now();

		// Start the progress bar.
		display_total(&shared_display, self.paths.len() as u64);

		// Process JPEGs and PNGs in parallel.
		let jpg_handle = || self.compress_jpgs(&shared_display);
		let png_handle = || self.compress_pngs(&shared_display);
		let (result_jpg, result_png) = rayon::join(jpg_handle, png_handle);

		// We're done with parallelization so can stop using tedious
		// wrapper functions.
		let mut display = shared_display.lock().unwrap();
		display.reset_bar();

		// Wrap it up.
		let saved: u64 = result_jpg + result_png;
		let elapsed: u64 = Lugar::time_diff(SystemTime::now(), start_time)?;
		Ok((saved, elapsed))
	}

	fn compress_jpgs(&self, shared_display: &Mutex<Pantalla>) -> u64 {
		// If we're skipping JPEGs, there's nothing to do.
		if self.skip == ImagenKind::Jpg {
			return 0;
		}

		// Find valid encoders.
		let mut encoders: Vec<&Imagen> = Vec::new();
		if ! self.mozjpeg.bin_path().is_none() {
			encoders.push(&self.mozjpeg);
		}
		if ! self.jpegoptim.bin_path().is_none() {
			encoders.push(&self.jpegoptim);
		}

		// If there aren't any encoders, we're done.
		if 0 == encoders.len() {
			return 0;
		}

		// Last thing, find all JPEG images, converting them into result
		// objects as we go.
		let mut images: Vec<Cosecha> = self.paths.clone()
			.into_par_iter()
			.filter_map(|x| {
				if x.has_extension("jpg") || x.has_extension("jpeg") {
					Some(Cosecha::new(x))
				}
				else {
					None
				}
			})
			.collect();
		if 0 == images.len() {
			return 0;
		}

		// Hold our combined savings.
		let mut saved: u64 = 0;

		// Loop the images.
		for mut v in &mut images {
			// Make sure it still exists. The encoders will re-check
			// this at each pass, but what's one more?
			if ! v.is_image() {
				display_tick(&shared_display);
				continue;
			}

			// The starting size.
			v.update();

			display_notice(
				&shared_display,
				format!(
					"{} {:>50} {}",
					Colour::Cyan.paint("Starting"),
					Pantalla::shorten_left(format!("{}", v.path()), 50),
					Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
				)
			);

			// Run through each encoder.
			for e in &encoders {
				if let Err(_) = e.compress(&mut v) {
					continue;
				}
			}

			// Report savings.
			if v.saved() > 0 {
				saved += v.saved();

				display_notice(
					&shared_display,
					format!(
						"{} {:>50} {} {}",
						Colour::Cyan.paint("Finished"),
						Pantalla::shorten_left(format!("{}", v.path()), 50),
						Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
						Colour::Green.bold().paint(format!(
							"-{}",
							Pantalla::nice_size(v.saved())),
						),
					)
				);
			}
			// There were no savings, but we can report we finished.
			else {
				display_notice(
					&shared_display,
					format!(
						"{} {:>50} {}",
						Colour::Cyan.paint("Finished"),
						Pantalla::shorten_left(format!("{}", v.path()), 50),
						Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
					)
				);
			}

			// Tick and move on.
			display_tick(&shared_display);
		}

		saved
	}

	fn compress_pngs(&self, shared_display: &Mutex<Pantalla>) -> u64 {
		// If we're skipping PNGs, there's nothing to do.
		if self.skip == ImagenKind::Png {
			return 0;
		}

		// Find valid encoders.
		let mut encoders: Vec<&Imagen> = Vec::new();
		if ! self.pngout.bin_path().is_none() {
			encoders.push(&self.pngout);
		}
		if ! self.oxipng.bin_path().is_none() {
			encoders.push(&self.oxipng);
		}
		if ! self.zopflipng.bin_path().is_none() {
			encoders.push(&self.zopflipng);
		}

		// If there aren't any encoders, we're done.
		if 0 == encoders.len() {
			return 0;
		}

		// Last thing, find all PNG images, converting them into result
		// objects as we go.
		let mut images: Vec<Cosecha> = self.paths.clone()
			.into_par_iter()
			.filter_map(|x| {
				if x.has_extension("png") {
					Some(Cosecha::new(x))
				}
				else {
					None
				}
			})
			.collect();
		if 0 == images.len() {
			return 0;
		}

		// Hold our combined savings.
		let mut saved: u64 = 0;

		// Loop the images.
		for mut v in &mut images {
			// Make sure it still exists. The encoders will re-check
			// this at each pass, but what's one more?
			if ! v.is_image() {
				display_tick(&shared_display);
				continue;
			}

			// The starting size.
			v.update();

			display_notice(
				&shared_display,
				format!(
					"{} {:>50} {}",
					Colour::Cyan.paint("Starting"),
					Pantalla::shorten_left(format!("{}", v.path()), 50),
					Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
				)
			);

			// Run through each encoder.
			for e in &encoders {
				if let Err(_) = e.compress(&mut v) {
					continue;
				}
			}

			// Report savings.
			if v.saved() > 0 {
				saved += v.saved();

				display_notice(
					&shared_display,
					format!(
						"{} {:>50} {} {}",
						Colour::Cyan.paint("Finished"),
						Pantalla::shorten_left(format!("{}", v.path()), 50),
						Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
						Colour::Green.bold().paint(format!(
							"-{}",
							Pantalla::nice_size(v.saved())),
						),
					)
				);
			}
			// There were no savings, but we can report we finished.
			else {
				display_notice(
					&shared_display,
					format!(
						"{} {:>50} {}",
						Colour::Cyan.paint("Finished"),
						Pantalla::shorten_left(format!("{}", v.path()), 50),
						Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
					)
				);
			}

			// Tick and move on.
			display_tick(&shared_display);
		}

		saved
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

	fn print_header(&self, display: &Pantalla) {
		// Try to be quiet.
		if display.show_quiet() {
			return;
		}

		let count: u64 = self.paths.len() as u64;
		let size: u64 = Lugar::du(&self.paths);

		display.plain(format!("
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

fn display_total(shared_display: &Mutex<Pantalla>, total: u64) {
	let mut display = shared_display.lock().unwrap();
	display.set_bar_total(total);
}

fn display_tick(shared_display: &Mutex<Pantalla>) {
	let mut display = shared_display.lock().unwrap();
	display.tick();
}

fn display_notice<S>(shared_display: &Mutex<Pantalla>, msg: S)
where S: Into<String> {
	let display = shared_display.lock().unwrap();

	// We're using plain instead of notice to switch up the formatting
	// a little bit, but we still want to restrict output to debugged
	// sessions.
	if display.show_debug() {
		let now = Lugar::local_now();

		display.plain(format!(
			"{} {}",
			Colour::Purple.paint(format!(
				"[{:02}:{:02}:{:02}]",
				now.hour(),
				now.minute(),
				now.second(),
			)),
			msg.into(),
		));
	}
}
