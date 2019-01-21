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
use std::sync::Mutex;
use std::time::SystemTime;



#[derive(Debug, Clone)]
/// Flaca Runtime Settings.
pub struct Ajustes {
	/// Skip files younger than this in minutes.
	min_age: u64,
	/// Skip files older than this in minutes.
	max_age: u64,
	/// Skip files smaller than this in bytes.
	min_size: u64,
	/// Skip files larger than this in bytes.
	max_size: u64,
	/// Skip either "jpg" or "png" image formats.
	skip: ImagenKind,
	/// A directory to log results to.
	log: Lugar,
	/// Location of the Jpegoptim binary.
	jpegoptim: Imagen,
	/// Location of the MozJPEG binary.
	mozjpeg: Imagen,
	/// Location of the Oxipng binary.
	oxipng: Imagen,
	/// Location of the Pngout binary.
	pngout: Imagen,
	/// Location of the Zopflipng binary.
	zopflipng: Imagen,
	/// Image paths to process.
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
	// -----------------------------------------------------------------
	// Init
	// -----------------------------------------------------------------

	/// Initialize Flaca
	///
	/// This parses command line arguments, generates help
	/// documentation, prints results to the screen, etc. It is
	/// basically our main().
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

		// Clean up raw paths a little bit.
		let mut raw: Vec<Lugar> = args
			.values_of("INPUT")
			.unwrap()
			.filter_map(|x| {
				let p: Lugar = Lugar::new(x);
				if p.exists() {
					Some(p)
				}
				else {
					None
				}
			})
			.collect();
		raw.par_sort();
		raw.dedup();

		// Build filters for Unix `find`, starting with extension. This
		// is much more performant than WalkDir when the file tree is
		// large, and about the same when it isn't.
		let mut find_args: Vec<String> = match skip {
			ImagenKind::Jpg => vec!["-iname".to_string(), "*.png".to_string()],
			ImagenKind::Png => vec![
				"(".to_string(),
				"-iname".to_string(),
				"*.jpg".to_string(),
				"-o".to_string(),
				"-iname".to_string(),
				"*.jpeg".to_string(),
				")".to_string(),
			],
			ImagenKind::None => vec![
				"(".to_string(),
				"-iname".to_string(),
				"*.jpg".to_string(),
				"-o".to_string(),
				"-iname".to_string(),
				"*.jpeg".to_string(),
				"-o".to_string(),
				"-iname".to_string(),
				"*.png".to_string(),
				")".to_string(),
			],
		};

		// We only want files.
		find_args.push("-type".to_string());
		find_args.push("f".to_string());

		// Age.
		if min_age > 60 {
			find_args.push("-mmin".to_string());
			find_args.push(format!("+{}", (min_age / 60) - 1).to_string());
		}
		if max_age > 60 {
			find_args.push("-mmin".to_string());
			find_args.push(format!("-{}", (max_age / 60)).to_string());
		}

		// Size.
		if min_size > 60 {
			find_args.push("-size".to_string());
			find_args.push(format!("+{}c", min_size - 1).to_string());
		}
		if max_size > 60 {
			find_args.push("-size".to_string());
			find_args.push(format!("-{}c", max_size).to_string());
		}

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
			paths: Lugar::walk_unix(&raw, &find_args),
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

		// Print the CLI header.
		out.print_header(&display);

		// No images, nothing to do.
		if 0 == out.paths.len() {
			display.error("No qualifying images were found.");
		}

		// Debug information? It can be helpful to see what settings are
		// being applied.
		if args.is_present("debug") {
			// Obviously we're debugging.
			display.notice("Debug mode enabled.");

			// Multithreading!
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

			// Are we skipping a format?
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

			// Give us a blank line.
			display.plain("");
		}

		// Start compressing!
		let shared_display = Mutex::new(display);
		if let Ok((saved, elapsed)) = out.compress(&shared_display) {
			// Another blank line.
			Pantalla::mutex_plain(&shared_display, "");
			Pantalla::mutex_success(
				&shared_display,
				format!("Finished in {}", Pantalla::nice_time(elapsed, false))
			);
			if saved > 0 {
				Pantalla::mutex_success(
					&shared_display,
					format!("Saved {}", Pantalla::nice_size(saved))
				);
			}
			else {
				Pantalla::mutex_warning(
					&shared_display,
					"No lossless savings were possible."
				);
			}
		}
		// Compression failed.
		else {
			Pantalla::mutex_error(&shared_display, "Compression failed.");
		}
	}

	// -----------------------------------------------------------------
	// Compression
	// -----------------------------------------------------------------

	/// Compress images.
	///
	/// JPEG and PNG formats are divided and conquered in separate
	/// threads. Generally speaking, JPEG processing should go much
	/// faster.
	fn compress(&self, shared_display: &Mutex<Pantalla>) -> Result<(u64, u64), Box<dyn Error>> {
		// Start your engines.
		let start_time = SystemTime::now();

		// Start the progress bar.
		Pantalla::mutex_set_bar_total(&shared_display, self.paths.len() as u64);

		// Process JPEGs and PNGs in parallel.
		let jpg_handle = || self.__compress_jpgs(&shared_display);
		let png_handle = || self.__compress_pngs(&shared_display);
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

	/// Compress the JPEG half.
	///
	/// This builds a vector of JPEG sources and figures out which
	/// encoders are available, then passes that along to the common
	/// method for handling.
	fn __compress_jpgs(&self, shared_display: &Mutex<Pantalla>) -> u64 {
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

		self.__compress_img(&mut images, &encoders, &shared_display)
	}

	/// Compress the PNG half.
	///
	/// This builds a vector of PNG sources and figures out which
	/// encoders are available, then passes that along to the common
	/// method for handling.
	fn __compress_pngs(&self, shared_display: &Mutex<Pantalla>) -> u64 {
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

		self.__compress_img(&mut images, &encoders, &shared_display)
	}

	/// Common compression routines.
	///
	/// This takes a vector of images and encoders and loops the loops.
	/// Results are sent to CLI and/or file log depending on settings.
	fn __compress_img(
		&self,
		images: &mut Vec<Cosecha>,
		encoders: &Vec<&Imagen>,
		shared_display: &Mutex<Pantalla>
	) -> u64 {
		// Hold our combined savings.
		let mut saved: u64 = 0;

		// Loop the images.
		for mut v in images {
			// Make sure it still exists. The encoders will re-check
			// this at each pass, but what's one more?
			if ! v.is_image() {
				Pantalla::mutex_tick(&shared_display);
				continue;
			}

			// The starting size.
			v.update();

			debug_notice(
				&shared_display,
				format!(
					"{} {:>50} {}",
					Colour::Cyan.paint("Starting"),
					Pantalla::shorten_left(format!("{}", v.path()), 50),
					Colour::Cyan.paint(Pantalla::nice_size(v.start_size())),
				)
			);

			// Run through each encoder.
			for e in encoders {
				if let Err(x) = e.compress(&mut v) {
					Pantalla::mutex_warning(&shared_display, x.to_string());
					continue;
				}
			}

			// Log the activity, maybe.
			if let Err(_) = v.log(&self.log) {
				Pantalla::mutex_warning(&shared_display, "Unable to log results to file");
			}

			// Report savings.
			if v.saved() > 0 {
				saved += v.saved();

				debug_notice(
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
				debug_notice(
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
			Pantalla::mutex_tick(&shared_display);
		}

		saved
	}

	// -----------------------------------------------------------------
	// Misc Output
	// -----------------------------------------------------------------

	/// CLI Header
	///
	/// This shows a cute ASCII art goat and some early summary
	/// information.
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



// ---------------------------------------------------------------------
// Misc Helpers
// ---------------------------------------------------------------------

/// Validate CLI Arg Min/Max Value
///
/// If present, it has to be u64-able.
fn validate_min_max(val: String) -> Result<(), String> {
	if let Ok(x) = val.parse::<u64>() {
		if x > 0 {
			return Ok(());
		}
	}

	Err("Value must be greater than zero.".to_string())
}

/// Parse CLI Age Min/Max Value
///
/// Age is requested in minutes, but internally we use seconds.
fn parse_min_max_age(val: Option<&str>) -> u64 {
	match val
		.unwrap_or("0")
		.parse::<u64>() {
			Ok(y) => y * 60,
			_ => 0,
		}
}

/// Parse CLI Size Min/Max Value
fn parse_min_max_size(val: Option<&str>) -> u64 {
	match val
		.unwrap_or("0")
		.parse::<u64>() {
			Ok(y) => y,
			_ => 0,
		}
}

/// Alternative Debug Notice. (Mutex)
///
/// We don't want to use Pantalla's "notice" formatting, but we also
/// want to avoid printing this unless debugging information has been
/// requested.
///
/// That's fine. Here's our own method.
fn debug_notice<S>(shared_display: &Mutex<Pantalla>, msg: S)
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
