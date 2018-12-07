#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]

extern crate ansi_term;
extern crate chrono;
extern crate clap;
extern crate indicatif;
#[macro_use]
extern crate lazy_static;
extern crate nix;

use ansi_term::{Colour::Blue, Colour::Green, Colour::Purple, Colour::Red, Colour::Yellow, Style};
use chrono::prelude::*;
use clap::{Arg, App};
use indicatif::{HumanDuration, HumanBytes, ProgressBar, ProgressStyle};
use nix::unistd::{chown, Uid, Gid};
use std::env::{current_exe, var, temp_dir};
use std::ffi::OsStr;
use std::fs::{metadata as Metadata, OpenOptions, Permissions, copy, remove_file, set_permissions};
use std::io::prelude::*;
use std::os::unix::fs::MetadataExt;
use std::path::{PathBuf, Path};
use std::process::{exit, Command, Stdio};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

// Build version.
const VERSION: &str = "0.2.0";

#[derive(Debug, Default, Clone)]
struct FlacaFile {
	path: String,
}

impl FlacaFile {
	/**
	 * Set From Path String
	 */
	fn new(path: String) -> FlacaFile {
		FlacaFile {
			path: path,
		}
	}

	/**
	 * As Path
	 */
	fn as_path(self) -> PathBuf {
		PathBuf::from(self.path)
	}

	/**
	 * Canonical Path
	 */
	fn canonical(self) -> Result<String, &'static str> {
		let path = self.as_path();
		if let Ok(x) = path.canonicalize() {
			return Ok(format!("{}", x.display()));
		}

		Err("File does not exist.")
	}

	/**
	 * Encoders
	 */
	fn encoders(self) -> Result<Vec<String>, &'static str> {
		if let Ok(ext) = self.extension() {
			let mut apps: Vec<String> = Vec::new();
			if "jpg" == ext {
				for i in &*JPG_APPS {
					apps.push(i.clone());
				}
			}
			else {
				for i in &*PNG_APPS {
					apps.push(i.clone());
				}
			}

			return Ok(apps);
		}

		Err("No encoders are supported for this file.")
	}

	/**
	 * Extension
	 */
	fn extension(self) -> Result<String, &'static str> {
		let path = self.as_path();
		if let Some(x) = path.extension() {
			if let Some(y) = OsStr::to_str(x) {
				let ext: String = y.to_lowercase();
				if "jpeg" == ext || "jpg" == ext {
					return Ok("jpg".to_string());
				}
				else if "png" == ext {
					return Ok("png".to_string());
				}
			}
		}

		Err("File is not a JPEG or PNG.")
	}

	/**
	 * Modification Time
	 */
	fn modified(self) -> Result<SystemTime, &'static str> {
		let path = self.as_path();
		if let Ok(x) = path.metadata().unwrap().modified() {
			return Ok(x);
		}

		Err("File does not exist.")
	}

	/**
	 * Modification Time (seconds)
	 */
	fn modified_secs(self) -> Result<u64, &'static str> {
		if let Ok(x) = self.modified() {
			if let Ok(y) = x.duration_since(UNIX_EPOCH) {
				return Ok(y.as_secs());
			}
		}

		Err("File does not exist.")
	}

	/**
	 * File Name
	 */
	fn name(self) -> Result<String, &'static str> {
		let path = self.as_path();
		if let Some(x) = path.file_name() {
			if let Some(y) = OsStr::to_str(x) {
				return Ok(y.to_string());
			}
		}

		Err("File does not exist.")
	}

	/**
	 * File Owner
	 */
	fn owner(self) -> Result<(Uid, Gid), &'static str> {
		let path = self.as_path();

		if let Ok(x) = path.metadata() {
			let uid = Uid::from_raw(x.uid());
			let gid = Gid::from_raw(x.gid());
			return Ok((uid, gid));
		}

		Err("File does not exist.")
	}

	/**
	 * File Permissions
	 */
	fn permissions(self) -> Result<Permissions, &'static str> {
		let path = self.as_path();

		if let Ok(x) = path.metadata() {
			return Ok(x.permissions());
		}

		Err("File does not exist.")
	}

	/**
	 * File Size
	 */
	fn size(self) -> Result<u64, &'static str> {
		let path = self.as_path();
		if let Ok(x) = path.metadata() {
			return Ok(x.len());
		}

		Err("File does not exist.")
	}

	/**
	 * Check Age
	 */
	fn check_age(self) -> Result<bool, &'static str> {
		let mtime: u64 = self.modified_secs()?;
		let start = SystemTime::now();
		let elapsed: u64 = match start.duration_since(UNIX_EPOCH) {
			Ok(x) => x.as_secs(),
			Err(_) => 0,
		};
		let max = *MAX_AGE.lock().unwrap();
		let min = *MIN_AGE.lock().unwrap();
		let age = elapsed - mtime;

		if (min > 0 && age < min) || (max > 0 && age > max) {
			return Err("File too large.");
		}

		Ok(true)
	}

	/**
	 * Check Size
	 */
	fn check_size(self) -> Result<bool, &'static str> {
		let size: u64 = self.size()?;
		let max = *MAX_SIZE.lock().unwrap();
		let min = *MIN_SIZE.lock().unwrap();

		if (min > 0 && size < min) || (max > 0 && size > max) {
			return Err("File too large.");
		}

		Ok(true)
	}

	/**
	 * Chmod
	 */
	fn chmod(self, perms: Permissions) -> Result<bool, &'static str> {
		if let Err(_) = set_permissions(self.to_owned().as_path(), perms) {
			eprintln!(
				"{} Unable to set ownership of {}.",
				Yellow.bold().paint("Warning:"),
				self.to_owned().path,
			);
		}

		Ok(true)
	}

	/**
	 * Chown
	 */
	fn chown(self, uid: Uid, gid: Gid) -> Result<bool, &'static str> {
		if let Err(_) = chown(&self.to_owned().as_path(), Some(uid), Some(gid)) {
			eprintln!(
				"{} Unable to set ownership of {}.",
				Yellow.bold().paint("Warning:"),
				self.to_owned().path,
			);
		}

		Ok(true)
	}

	/**
	 * Copy
	 */
	fn copy(self, dest: String) -> Result<FlacaFile, &'static str> {
		let path = self.as_path();
		let path2 = Path::new(&dest);

		if path.is_file() {
			// We might have to delete first.
			if path2.exists() {
				if let Err(_) = remove_file(path2) {
					eprintln!(
						"{} Could not copy file; destination file exists.",
						Yellow.bold().paint("Warning:"),
					);

					return Err("File could not be copied.");
				}
			}

			// Try to copy.
			if let Err(_) = copy(path, path2) {
				eprintln!(
					"{} Could not copy file.",
					Yellow.bold().paint("Warning:"),
				);

				return Err("File could not be copied.");
			}

			// Send a Flaca back-a.
			return Ok(FlacaFile::new(dest.to_owned()));
		}

		Err("File does not exist.")
	}

	/**
	 * Remove
	 */
	fn remove(self) -> Result<bool, &'static str> {
		let path = self.as_path();
		if path.exists() {
			if let Err(_) = remove_file(path) {
				return Err("File could not be removed.");
			}
		}

		Ok(true)
	}

	/**
	 * Working Copy
	 *
	 * Like copy, except the destination is computed automatically.
	 */
	fn working_copy(self) -> Result<FlacaFile, &'static str> {
		// We need a valid extension.
		if let Ok(ext) = self.to_owned().extension() {
			let file = self.to_owned().name()?;
			let mut num: u64 = 0;
			let dir = PathBuf::from(&temp_dir());
			if dir.is_dir() {
				// This is easier if we have a proper string.
				if let Ok(base) = dir.canonicalize() {
					// Generate a stub name.
					let mut out_name: String = format!(
						"{}/{}.__flaca{}.{}",
						base.display(),
						file,
						num.to_string(),
						ext
					);

					// Repeat until we have a file that is unique.
					while Path::new(&out_name).exists() {
						num = num + 1;

						out_name = format!(
							"{}/{}.__flaca{}.{}",
							base.display(),
							file,
							num.to_string(),
							ext
						);
					}

					if let Ok(x) = self.copy(out_name) {
						return Ok(x);
					}
				}
			}
		}

		Err("Could not create working copy.")
	}
}

lazy_static! {
	/// Dry run; do not override any files.
	static ref DRY_RUN: Mutex<bool> = Mutex::new(false);

	/// Images to process.
	static ref IMAGES: Mutex<Vec<FlacaFile>> = Mutex::new(Vec::new());

	/// Path to log file.
	static ref LOG: Mutex<String> = Mutex::new("".to_string());

	/// Maximum file age in seconds.
	static ref MAX_AGE: Mutex<u64> = Mutex::new(0);

	/// Maximum file size in bytes.
	static ref MAX_SIZE: Mutex<u64> = Mutex::new(0);

	/// Minimum file age in seconds.
	static ref MIN_AGE: Mutex<u64> = Mutex::new(0);

	/// Minimum file size in bytes.
	static ref MIN_SIZE: Mutex<u64> = Mutex::new(0);

	/// Suppress STDOUT.
	static ref QUIET: Mutex<bool> = Mutex::new(false);

	/// Total disk usage at the end.
	static ref SIZE_NEW: Mutex<u64> = Mutex::new(0);

	/// Total disk usage at the start.
	static ref SIZE_OLD: Mutex<u64> = Mutex::new(0);

	/// Skip a format (jpg or png) when e.g. recursing directories.
	static ref SKIP: Mutex<String> = Mutex::new("".to_string());

	/// Alternate jpegoptim binary path.
	static ref JPEGOPTIM: Mutex<String> = Mutex::new("jpegoptim".to_string());

	/// Alternate MozJPEG binary path.
	static ref JPEGTRAN: Mutex<String> = Mutex::new("jpegtran".to_string());

	/// Alternate Oxipng binary path.
	static ref OXIPNG: Mutex<String> = Mutex::new("oxipng".to_string());

	/// Alternate pngout binary path.
	static ref PNGOUT: Mutex<String> = Mutex::new("pngout".to_string());

	/// Alternate Zopflipng binary path.
	static ref ZOPFLIPNG: Mutex<String> = Mutex::new("zopflipng".to_string());

	// Binaries to run against a PNG, in order.
	static ref PNG_APPS: Vec<String> = vec![
		"pngout".to_string(),
		"oxipng".to_string(),
		"zopflipng".to_string(),
	];

	// Binaries to run against a JPEG, in order.
	static ref JPG_APPS: Vec<String> = vec![
		"jpegtran".to_string(),
		"jpegoptim".to_string(),
	];
}



// ---------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------

/**
 * Main
 */
fn main() {
	// Check program requirements.
	check_args();
	check_requirements();

	// Parse runtime arguments.
	if 0 == get_images_len() {
		eprintln!(
			"{} No qualifying images were found.",
			Red.bold().paint("Error:")
		);

		exit(1);
	}

	// Process!
	if false == *QUIET.lock().unwrap() {
		print_header();
		compress_images();
	}
	else {
		compress_images_quiet();
	}

	exit(0);
}

/**
 * Loop W/ Progress
 */
fn compress_images() {
	let start = SystemTime::now();

	// Main image thread count.
	let progress_images = ProgressBar::new(get_images_len());
	progress_images.set_style(
		ProgressStyle::default_bar()
		.template("[{elapsed_precise}] [{bar:40.cyan/blue}]  {pos:.cyan.bold}/{len:.blue.bold}  {percent:.bold}%  {msg}")
		.progress_chars("##-")
	);

	// Loop the images!
	for image in &*IMAGES.lock().unwrap() {
		// Announce we've started.
		progress_images.println(format!(
			"{} {}",
			Purple.bold().paint(get_local_now().to_rfc3339()),
			image.to_owned().canonical().unwrap(),
		));

		let apps = image.to_owned().encoders().unwrap();

		// Keep track of how much we've saved this pass.
		let mut saved: u64 = 0;
		let mut elapsed: u64 = 0;

		// Run what needs running.
		for app in apps {
			let mut _size_new = SIZE_NEW.lock().unwrap();

			progress_images.tick();
			if let Ok((rsaved, relapsed)) = compress_image(image.to_owned(), &app) {
				saved += rsaved;
				*_size_new -= rsaved;

				let total_saved = *SIZE_OLD.lock().unwrap() - *_size_new;
				if total_saved > 0 {
					progress_images.set_message(&format!(
						"{}",
						Green.bold().paint(format!("-{}", HumanBytes(total_saved))),
					));
				}
				else {
					progress_images.set_message("");
				}

				elapsed += relapsed;
			}
		}

		// Log before we move on?
		if "".to_string() != *LOG.lock().unwrap() {
			log_image(
				image.to_owned(),
				saved,
				elapsed,
			);
		}

		progress_images.inc(1);
	}

	progress_images.finish_and_clear();

	// We're done!
	let end = SystemTime::now();
	let end_since = end.duration_since(start).expect("Time is meaningless.");

	println!(
		"{} {}",
		Green.bold().paint("Finished:"),
		HumanDuration(end_since),
	);

	if *SIZE_OLD.lock().unwrap() != *SIZE_NEW.lock().unwrap() {
		println!(
			"{} {}",
			Green.bold().paint("Saved:"),
			HumanBytes(*SIZE_OLD.lock().unwrap() - *SIZE_NEW.lock().unwrap()),
		);
	}
	else {
		println!(
			"{} {}",
			Yellow.bold().paint("Saved:"),
			"0B",
		);
	}
}

/**
 * Loop w/o Progress
 */
fn compress_images_quiet() {
	// Loop the images!
	for image in &*IMAGES.lock().unwrap() {
		let apps = image.to_owned().encoders().unwrap();

		// Keep track of how much we've saved this pass.
		let mut saved: u64 = 0;
		let mut elapsed: u64 = 0;

		// Run what needs running.
		for app in apps {
			let mut _size_new = SIZE_NEW.lock().unwrap();

			if let Ok((rsaved, relapsed)) = compress_image(image.to_owned(), &app) {
				saved += rsaved;
				*_size_new -= rsaved;

				elapsed += relapsed;
			}
		}

		// Log before we move on?
		if "".to_string() != *LOG.lock().unwrap() {
			log_image(
				image.to_owned(),
				saved,
				elapsed,
			);
		}
	}
}



// ---------------------------------------------------------------------
// Set Up
// ---------------------------------------------------------------------

/**
 * Check Requirements
 *
 * Flaca is just a launcher; the image encoders must be natively
 * installed on the user's system.
 */
fn check_requirements() {
	let mut error: bool = false;

	let apps = [
		("jpegoptim", &*JPEGOPTIM.lock().unwrap()),
		("MozJPEG", &*JPEGTRAN.lock().unwrap()),
		("oxipng", &*OXIPNG.lock().unwrap()),
		("pngout", &*PNGOUT.lock().unwrap()),
		("zopflipng", &*ZOPFLIPNG.lock().unwrap()),
	];

	for (name, app) in apps.iter() {
		if false == check_requirement(app) {
			error = true;
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(name.to_string())
			);
		}
	}

	if true == error {
		exit(1);
	}
}

/**
 * Check Requirement
 *
 * Ensure a given binary is found in the $PATH.
 */
fn check_requirement(app: &str) -> bool {
	// This could be a full path.
	let path = Path::new(app);
	if path.is_file() {
		return true;
	}

	// If not, let's see if it is found in any of the $PATH dirs.
	if let Ok(path) = var("PATH") {
		for p in path.split(":") {
			let p_str = format!("{}/{}", p, app);
			if Metadata(p_str).is_ok() {
				return true;
			}
		}
	}

	false
}

/**
 * Get App Command
 */
fn get_app_path(app: &str) -> String {
	let path: String;

	match app {
		"jpegoptim" => path = format!("{}", &*JPEGOPTIM.lock().unwrap()),
		"jpegtran" => path = format!("{}", &*JPEGTRAN.lock().unwrap()),
		"oxipng" => path = format!("{}", &*OXIPNG.lock().unwrap()),
		"pngout" => path = format!("{}", &*PNGOUT.lock().unwrap()),
		"zopflipng" => path = format!("{}", &*ZOPFLIPNG.lock().unwrap()),
		_ => {
			eprintln!(
				"{} Invalid binary {}.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(app)
			);
			exit(1);
		}
	};

	path
}

/**
 * Runtime Arguments
 *
 * This both declares available runtime arguments and helps crunch them
 * into a consistent, usable frame of reference.
 */
fn check_args() {
	// Set up runtime arguments.
	let args = App::new("Flaca")
		.version(VERSION)
		.author("Blobfolio, LLC <hello@blobfolio.com>")
		.about("Losslessly compress the mierda out of JPEG and PNG images.")
		.arg(Arg::with_name("dry_run")
			.short("d")
			.long("dry_run")
			.alias("dry-run")
			.help("Conduct a trial run without altering your images.")
		)
		.arg(Arg::with_name("log")
			.short("l")
			.long("log")
			.help("Log operations to this location.")
			.value_name("PATH")
			.takes_value(true)
		)
		.arg(Arg::with_name("min_age")
			.long("min_age")
			.alias("min-age")
			.help("Ignore files younger than this.")
			.value_name("MINUTES")
			.takes_value(true)
			.validator(check_min_max_args)
		)
		.arg(Arg::with_name("max_age")
			.long("max_age")
			.alias("max-age")
			.help("Ignore files older than this.")
			.value_name("MINUTES")
			.takes_value(true)
			.validator(check_min_max_args)
		)
		.arg(Arg::with_name("min_size")
			.long("min_size")
			.alias("min-size")
			.help("Ignore files smaller than this.")
			.value_name("BYTES")
			.takes_value(true)
			.validator(check_min_max_args)
		)
		.arg(Arg::with_name("max_size")
			.long("max_size")
			.alias("max-size")
			.help("Ignore files larger than this.")
			.value_name("BYTES")
			.takes_value(true)
			.validator(check_min_max_args)
		)
		.arg(Arg::with_name("quiet")
			.short("q")
			.long("quiet")
			.help("Suppress STDOUT. This has no effect on errors.")
		)
		.arg(Arg::with_name("skip")
			.short("s")
			.long("skip")
			.help("Skip images of this type.")
			.value_name("FORMAT")
			.takes_value(true)
			.possible_values(&["jpeg", "jpg", "png"])
		)
		.arg(Arg::with_name("jpegoptim")
			.long("jpegoptim")
			.help("Alternate binary path for jpegoptim.")
			.value_name("BIN")
			.takes_value(true)
		)
		.arg(Arg::with_name("mozjpeg")
			.long("mozjpeg")
			.alias("jpegtran")
			.help("Alternate binary path for MozJPEG.")
			.value_name("BIN")
			.takes_value(true)
		)
		.arg(Arg::with_name("oxipng")
			.long("oxipng")
			.help("Alternate binary path for oxipng.")
			.value_name("BIN")
			.takes_value(true)
		)
		.arg(Arg::with_name("pngout")
			.long("pngout")
			.help("Alternate binary path for pngout.")
			.value_name("BIN")
			.takes_value(true)
		)
		.arg(Arg::with_name("zopflipng")
			.long("zopflipng")
			.help("Alternate binary path for zopflipng.")
			.value_name("BIN")
			.takes_value(true)
		)
		.arg(Arg::with_name("INPUT")
			.index(1)
			.help("Images(s) to crunch or where to find them.")
			.required(true)
			.multiple(true)
			.use_delimiter(false)
		)
		.after_help("REQUIRED OPTIMIZERS:
    jpegoptim <https://github.com/tjko/jpegoptim>
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    oxipng    <https://github.com/shssoichiro/oxipng>
    pngout    <http://advsys.net/ken/utils.htm>
    zopflipng <https://github.com/google/zopfli>
		")
		.get_matches();

	// Dry run.
	let mut _dry_run = DRY_RUN.lock().unwrap();
	*_dry_run = args.is_present("dry_run");

	// Quiet mode.
	let mut _quiet = QUIET.lock().unwrap();
	*_quiet = args.is_present("quiet");

	// Min age.
	if let Some(x) = args.value_of("min_age") {
		// This program accepts age in minutes, but internally we will
		// be using seconds.
		let mut _min_age = MIN_AGE.lock().unwrap();
		*_min_age = x.parse::<u64>().unwrap() * 60;
	}

	// Max age.
	if let Some(x) = args.value_of("max_age") {
		// This program accepts age in minutes, but internally we will
		// be using seconds.
		let mut _max_age = MAX_AGE.lock().unwrap();
		*_max_age = x.parse::<u64>().unwrap() * 60;
	}

	// Min size.
	if let Some(x) = args.value_of("min_size") {
		let mut _min_size = MIN_SIZE.lock().unwrap();
		*_min_size = x.parse::<u64>().unwrap();
	}

	// Max size.
	if let Some(x) = args.value_of("max_size") {
		let mut _max_size = MAX_SIZE.lock().unwrap();
		*_max_size = x.parse::<u64>().unwrap();
	}

	// Log path.
	if let Some(x) = args.value_of("log") {
		let mut _log = LOG.lock().unwrap();
		*_log = x.parse::<String>().unwrap();
	}

	// Are we skipping a format?
	if let Some(x) = args.value_of("skip") {
		let mut _skip = SKIP.lock().unwrap();
		*_skip = x.parse::<String>().unwrap();
		if "jpeg" == *_skip {
			*_skip = "jpg".to_string();
		}
	}

	// Binary paths.
	let me_dir = &current_exe().unwrap();
	let flaca_dir = Path::new(me_dir).parent().unwrap();
	let flaca_dir2 = flaca_dir.join("bin");

	// Assume relative binary directory.
	let mut bin_dir = Path::new(&flaca_dir2);

	// That failing, there could be a shared folder.
	if ! bin_dir.is_dir() {
		bin_dir = Path::new("/usr/share/flaca");
	}

	// jpegoptim.
	if let Some(x) = args.value_of("jpegoptim") {
		let app = PathBuf::from(x);
		if ! app.is_file() {
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(x)
			);

			exit(1);
		}

		let mut _jpegoptim = JPEGOPTIM.lock().unwrap();
		if let Ok(x) = app.canonicalize() {
			*_jpegoptim = format!("{}", x.display());
		}
	}
	else if bin_dir.is_dir() {
		let app = bin_dir.join("jpegoptim");
		if app.is_file() {
			let mut _jpegoptim = JPEGOPTIM.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_jpegoptim = format!("{}", x.display());
			}
		}
	}

	// MozJPEG.
	if let Some(x) = args.value_of("jpegtran") {
		let app = PathBuf::from(x);
		if ! app.is_file() {
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(x)
			);

			exit(1);
		}

		let mut _jpegtran = JPEGTRAN.lock().unwrap();
		if let Ok(x) = app.canonicalize() {
			*_jpegtran = format!("{}", x.display());
		}
	}
	else if bin_dir.is_dir() {
		let app = bin_dir.join("jpegtran");
		if app.is_file() {
			let mut _jpegtran = JPEGTRAN.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_jpegtran = format!("{}", x.display());
			}
		}
	}
	// This one could also be in /opt.
	else {
		let app = PathBuf::from("/opt/mozjpeg/bin/jpegtran");
		if app.is_file() {
			let mut _jpegtran = JPEGTRAN.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_jpegtran = format!("{}", x.display());
			}
		}
	}

	// Oxipng.
	if let Some(x) = args.value_of("oxipng") {
		let app = PathBuf::from(x);
		if ! app.is_file() {
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(x)
			);

			exit(1);
		}

		let mut _oxipng = OXIPNG.lock().unwrap();
		if let Ok(x) = app.canonicalize() {
			*_oxipng = format!("{}", x.display());
		}
	}
	else if bin_dir.is_dir() {
		let app = bin_dir.join("oxipng");
		if app.is_file() {
			let mut _oxipng = OXIPNG.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_oxipng = format!("{}", x.display());
			}
		}
	}

	// pngout.
	if let Some(x) = args.value_of("pngout") {
		let app = PathBuf::from(x);
		if ! app.is_file() {
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(x)
			);

			exit(1);
		}

		let mut _pngout = PNGOUT.lock().unwrap();
		if let Ok(x) = app.canonicalize() {
			*_pngout = format!("{}", x.display());
		}
	}
	else if bin_dir.is_dir() {
		let app = bin_dir.join("pngout");
		if app.is_file() {
			let mut _pngout = PNGOUT.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_pngout = format!("{}", x.display());
			}
		}
	}

	// Zopflipng.
	if let Some(x) = args.value_of("zopflipng") {
		let app = PathBuf::from(x);
		if ! app.is_file() {
			eprintln!(
				"{} {} was not found.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(x)
			);

			exit(1);
		}

		let mut _zopflipng = ZOPFLIPNG.lock().unwrap();
		if let Ok(x) = app.canonicalize() {
			*_zopflipng = format!("{}", x.display());
		}
	}
	else if bin_dir.is_dir() {
		let app = bin_dir.join("zopflipng");
		if app.is_file() {
			let mut _zopflipng = ZOPFLIPNG.lock().unwrap();
			if let Ok(x) = app.canonicalize() {
				*_zopflipng = format!("{}", x.display());
			}
		}
	}

	// Now we have enough information to pull our images.
	let mut _images = IMAGES.lock().unwrap();
	*_images = clean_images(check_images(
		args
			.values_of("INPUT")
			.unwrap()
			.map(PathBuf::from)
			.collect(),
	));

	// Let's go through and add up the sizes.
	let mut _size_old = SIZE_OLD.lock().unwrap();
	let mut _size_new = SIZE_NEW.lock().unwrap();

	for image in &*_images {
		if let Ok(size) = image.to_owned().size() {
			*_size_old += size;
			*_size_new += size;
		}
	}
}

/**
 * Callback for Min/Max flags
 *
 * Min/max age and size arguments all work the same way, and if
 * provided, must be greater than 0.
 */
fn check_min_max_args(x: String) -> Result<(), String> {
    match x.parse::<u64>() {
		Ok(y) => {
			if y > 0 {
				Ok(())
			} else {
				Err("Value must be at least 1.".to_string())
			}
		}
		Err(_) => Err("Value must be at least 1.".to_string())
	}
}

/**
 * Collect Images
 *
 * Any number of files or directories could be passed to this app; we
 * need to analyze those and pull out qualifying images.
 */
fn check_images(files: Vec<PathBuf>) -> Vec<String> {
	let mut out = Vec::new();

	for image in files {
		// Recurse directories.
		if image.is_dir() {
			let files = image
				.read_dir()
				.unwrap()
				.map(|x| x.unwrap().path().to_owned())
				.collect();
			out.extend(check_images(files));
		}
		// Just a regular old file.
		else if image.is_file() {
			// Push it. We'll check applicability later.
			out.push(format!("{}", image.canonicalize().unwrap().display()));
		}
	}

	// Done!
	out
}

/**
 * Clean Image List
 *
 * To reduce operations, we'll wait and sort/dedup image paths once at
 * the end of the run.
 */
fn clean_images(mut images: Vec<String>) -> Vec<FlacaFile> {
	// Sort and dedup.
	images.sort();
	images.dedup();

	// Recompile as a vector of PathBufs.
	let mut out: Vec<FlacaFile> = Vec::new();
	for image in &images {
		let tmp = FlacaFile::new(image.to_string());

		// Check extensions first.
		if let Ok(ext) = tmp.to_owned().extension() {
			if ext == *SKIP.lock().unwrap() {
				continue;
			}
		}
		else {
			continue;
		}

		// Check age if applicable.
		if let Err(_) = tmp.to_owned().check_age() {
			continue;
		}

		// Check size if applicable.
		if let Err(_) = tmp.to_owned().check_size() {
			continue;
		}

		out.push(tmp);
	}

	// Done!
	out
}



// ---------------------------------------------------------------------
// Compression
// ---------------------------------------------------------------------

/**
 * Compress Image
 */
fn compress_image(
	image: FlacaFile,
	app: &str,
) -> Result<(u64, u64), &'static str> {
	let start = SystemTime::now();
	let size_old: u64 = image.to_owned().size().unwrap_or(0);

	let working1: FlacaFile = image.to_owned().working_copy().unwrap();
	let mut working2: String = "".to_string();

	// These apps need a second working path.
	if "jpegtran" == app || "zopflipng" == app {
		working2 = format!("{}.bak", working1.to_owned().canonical().unwrap_or("".to_string()));
	}

	let mut saved: u64 = 0;

	let mut com = Command::new(get_app_path(app));
	match app {
		"jpegoptim" => {
			com.arg("-q");
			com.arg("-f");
			com.arg("--strip-all");
			com.arg("--all-progressive");
			com.arg(working1.to_owned().canonical().unwrap());
		}
		"jpegtran" => {
			com.arg("-copy");
			com.arg("none");
			com.arg("-optimize");
			com.arg("-progressive");
			com.arg("-outfile");
			com.arg(&working2);
			com.arg(working1.to_owned().canonical().unwrap());
		}
		"oxipng" => {
			com.arg("-s");
			com.arg("-q");
			com.arg("--fix");
			com.arg("-o");
			com.arg("6");
			com.arg("-i");
			com.arg("0");
			com.arg(working1.to_owned().canonical().unwrap());
		}
		"pngout" => {
			com.arg(working1.to_owned().canonical().unwrap());
			com.arg("-q");
		}
		"zopflipng" => {
			com.arg("-m");
			com.arg(working1.to_owned().canonical().unwrap());
			com.arg(&working2);
		}
		_ => {
			eprintln!(
				"{} Invalid binary {}.",
				Red.bold().paint("Error:"),
				Style::new().bold().paint(app)
			);
			exit(1);
		}
	}

	// Run it!
	if let Err(_) = com
		.stdout(Stdio::piped())
		.stderr(Stdio::piped())
		.output() {
		eprintln!(
			"{} Could not execute {}.",
			Red.bold().paint("Error:"),
			Style::new().bold().paint(app)
		);
		exit(1);
	}

	// We might have to swap working files.
	if working2 != "" {
		let tmp = FlacaFile::new(working2);
		if let Err(_) = tmp.to_owned().copy(working1.to_owned().canonical().unwrap_or("".to_string())) {
			return Err("No change.")
		}
		if let Err(_) = tmp.remove() {
			eprintln!(
				"{} Temporary files left over.",
				Yellow.bold().paint("Warning:"),
			);
		}
	}

	// Calculate size savings.
	let size_new = working1.to_owned().size().unwrap();
	if size_new < size_old {
		saved = size_old - size_new;

		// Replace the original?
		if false == *DRY_RUN.lock().unwrap() {
			// Make sure we can grab the owner/group.
			if let Ok((uid, gid)) = image.to_owned().owner() {
				// And permissions.
				if let Ok(perms) = image.to_owned().permissions() {
					// First operation: copy it.
					if let Err(_) = working1.to_owned().copy(image.to_owned().canonical().unwrap_or("".to_string())) {
						return Err("No change");
					}

					// Fix ownership.
					if let Err(_) = image.to_owned().chown(uid, gid) {
						eprintln!(
							"{} Unable to set file ownership.",
							Yellow.bold().paint("Warning:"),
						);
					}

					// Fix permissions.
					if let Err(_) = image.to_owned().chmod(perms) {
						eprintln!(
							"{} Unable to set file permissions.",
							Yellow.bold().paint("Warning:"),
						);
					}
				}
			}
		}
	}

	// Remove the working file.
	if let Err(_) = working1.remove() {
		eprintln!(
			"{} Temporary files left over.",
			Yellow.bold().paint("Warning:"),
		);
	}

	// Calculate duration.
	let end = SystemTime::now();
	let elapsed: u64 = match end.duration_since(start) {
		Ok(x) => x.as_secs(),
		Err(_) => 0,
	};

	if saved > 0 {
		return Ok((saved, elapsed));
	}

	Err("No change.")
}



// ---------------------------------------------------------------------
// Output
// ---------------------------------------------------------------------

/**
 * Print Header
 */
fn print_header() {
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
		Purple.bold().paint("Flaca"),
		Style::new().bold().paint(format!("v{}", VERSION)),
		Blue.bold().paint("Images:"),
		get_images_len(),
		Blue.bold().paint("Space:"),
		HumanBytes(*SIZE_OLD.lock().unwrap()),
		"Ready, Set, Goat!",
	);
}

/**
 * Log Image Details
 *
 * Save image compression activity to log.
 */
fn log_image(
	image: FlacaFile,
	saved: u64,
	elapsed: u64,
) {
	// Assume whatever this was just happened.
	let datetime = get_local_now();

	// Derive the old and new sizes.
	let size_new: u64 = image.to_owned().size().unwrap_or(0);
	let size_old: u64 = size_new + saved;

	// Build a simple status.
	let status: String = "No change.".to_string();
	if saved > 0 {
		format!("Saved {} bytes in {} seconds.", saved, elapsed);
	}

	let mut file = OpenOptions::new()
		.write(true)
		.append(true)
		.create(true)
		.open(Path::new(&*LOG.lock().unwrap().clone()))
		.unwrap();

    if let Err(_) = writeln!(
    	file,
    	"{} \"{}\" {} {} {}",
		datetime.to_rfc3339(),
		image.canonical().unwrap(),
		size_old,
		size_new,
		status,
    ) {
    	eprintln!(
			"{} Results could not be written to {}.",
			Red.bold().paint("Error:"),
			*LOG.lock().unwrap()
		);
    }
}



// ---------------------------------------------------------------------
// Misc Helpers
// ---------------------------------------------------------------------

/**
 * Get Images Length
 *
 * Rust is being stupid.
 */
fn get_images_len() -> u64 {
	let _images = &*IMAGES.lock().unwrap();
	_images.len() as u64
}

/**
 * Get Local Now
 */
fn get_local_now() -> chrono::DateTime<chrono::Local> {
	let start = SystemTime::now();
	let start_since = start.duration_since(UNIX_EPOCH).expect("Time is meaningless.");

	Local.timestamp(start_since.as_secs() as i64, 0)
}
