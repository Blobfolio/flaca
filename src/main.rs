#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]



extern crate ansi_term;
extern crate chrono;
extern crate clap;
extern crate indicatif;
#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate regex;



use chrono::TimeZone;
use std::io::Write;
use std::os::unix::fs::MetadataExt;



// ---------------------------------------------------------------------
// Definitions
// ---------------------------------------------------------------------

/// Build version.
const VERSION: &'static str = env!("CARGO_PKG_VERSION");

/// Encoder.
#[derive(Debug, PartialEq, Copy, Clone)]
enum FlacaEncoder {
	Jpegoptim,
	Mozjpeg,
	Oxipng,
	Pngout,
	Zopflipng,
}

impl std::fmt::Display for FlacaEncoder {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match *self {
				FlacaEncoder::Jpegoptim => "jpegoptim",
				FlacaEncoder::Mozjpeg => "MozJPEG",
				FlacaEncoder::Oxipng => "oxipng",
				FlacaEncoder::Pngout => "pngout",
				FlacaEncoder::Zopflipng => "zopflipng",
			}
		)
	}
}

impl FlacaEncoder {
	/// Get path to encoder binary.
	fn bin_path(&self, path: Option<std::path::PathBuf>) -> Option<std::path::PathBuf> {
		// User-supplied path.
		if let Some(x) = path {
			if x.is_file() {
				return Some(x);
			}
		}

		// Flaca bin directory.
		let bin = format!("{}", match *self {
			FlacaEncoder::Jpegoptim => "jpegoptim",
			FlacaEncoder::Mozjpeg => "jpegtran",
			FlacaEncoder::Oxipng => "oxipng",
			FlacaEncoder::Pngout => "pngout",
			FlacaEncoder::Zopflipng => "zopflipng",
		});
		let mut test = std::path::PathBuf::from(format!("/usr/share/flaca/{}", bin));
		if test.is_file() {
			return Some(test);
		}

		// MozJPEG has an extra possible path.
		if "jpegtran" == bin {
			test = std::path::PathBuf::from("/opt/mozjpeg/bin/jpegtran");
			if test.is_file() {
				return Some(test);
			}
		}

		// Maybe it is in the user path?
		if let Ok(path) = std::env::var("PATH") {
			for p in path.split(":") {
				let test = std::path::PathBuf::from(format!("{}/{}", p, bin));
				if test.is_file() {
					return Some(test);
				}
			}
		}

		None
	}
}

/// Errors.
#[derive(Debug, PartialEq, Clone)]
enum FlacaError {
	/// Command failed.
	CommandFailed,
	/// Debug message.
	Debug,
	/// Invalid file.
	FileInvalid,
	/// Could not log results.
	LogFailed,
	/// JPEG dependencies are missing.
	NoDepsJpg,
	/// PNG dependencies are missing.
	NoDepsPng,
	/// No qualifying images were found.
	NoFiles,
	/// Unable to set file permissions.
	PermsFailed,
	/// Time is meaningless.
	TimeFailed,
	/// The temporary directory might contain leftover files.
	TmpFailed,
	/// Value must be at least one.
	UsizeMinMax,
}

impl std::fmt::Display for FlacaError {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match *self {
				FlacaError::CommandFailed => "Command failed.",
				FlacaError::Debug => "",
				FlacaError::FileInvalid => "Invalid file.",
				FlacaError::LogFailed => "Could not log results.",
				FlacaError::NoDepsJpg => "JPEG dependencies are missing.",
				FlacaError::NoDepsPng => "PNG dependencies are missing.",
				FlacaError::NoFiles => "No qualifying images were found.",
				FlacaError::PermsFailed => "Unable to set file permissions.",
				FlacaError::TimeFailed => "Time is meaningless.",
				FlacaError::TmpFailed => "The temporary directory might contain leftover files.",
				FlacaError::UsizeMinMax => "Value must be at least one.",
			}
		)
	}
}

impl FlacaError {
	/// Convert to a String.
	fn to_string(&self, more: Option<String>) -> String {
		if more.is_some() {
			if self == &FlacaError::Debug {
				return format!(
					"[{}] {} {}",
					ansi_term::Colour::Blue.bold().paint(get_local_now().to_rfc3339()),
					ansi_term::Colour::Blue.bold().paint("Debug:"),
					more.unwrap()
				);
			}

			return format!(
				"{}\n{} {}",
				self,
				ansi_term::Colour::Blue.bold().paint("Reference:"),
				more.unwrap(),
			);
		}

		format!("{}", self)
	}

	/// Print a warning to STDERR.
	fn warn(&self, more: Option<String>) {
		eprintln!(
			"{} {}\n",
			ansi_term::Colour::Yellow.bold().paint("Warning"),
			self.to_string(more),
		);
	}

	/// Print an error to STDERR and exit.
	fn error(&self, more: Option<String>) {
		eprintln!(
			"{} {}",
			ansi_term::Colour::Red.bold().paint("Warning"),
			self.to_string(more),
		);

		std::process::exit(1);
	}

	/// Debug message.
	fn debug(&self, more: Option<String>, output: Option<&indicatif::ProgressBar>) {
		if let Some(x) = output {
			x.println(format!("{}", self.to_string(more)));
		}
		else {
			println!("{}", self.to_string(more));
		}
	}
}

/// Image types.
#[derive(Debug, PartialEq, Copy, Clone)]
enum FlacaImageType {
	/// JPEG.
	Jpg,
	/// PNG.
	Png,
}

impl std::fmt::Display for FlacaImageType {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		write!(
			f,
			"{}",
			match *self {
				FlacaImageType::Jpg => "jpg",
				FlacaImageType::Png => "png",
			}
		)
	}
}

impl FlacaImageType {
	/// Get image type by extension.
	fn from(path: std::path::PathBuf) -> Option<FlacaImageType> {
		lazy_static! {
			static ref expr: regex::Regex = regex::Regex::new(r"\.(?P<ext>jpe?g|png)$").unwrap();
		}

		// Generate a full path.
		let full = get_file_canonical(path);
		if full.is_none() {
			return None;
		}

		let lower = full.unwrap().to_lowercase();
		if let Some(matches) = expr.captures(&lower) {
			return match &matches["ext"] {
				"jpg" => Some(FlacaImageType::Jpg),
				"jpeg" => Some(FlacaImageType::Jpg),
				"png" => Some(FlacaImageType::Png),
				_ => None
			};
		}

		None
	}

	/// Get encoders for type.
	fn encoders(&self, opts: FlacaSettings) -> Option<Vec<(FlacaEncoder, String)>> {
		let out: Vec<(FlacaEncoder, String)> = match *self {
			FlacaImageType::Jpg => {
				let mut tmp = Vec::new();

				// MozJPEG.
				if let Some(x) = opts.use_mozjpeg {
					if let Some(y) = get_file_canonical(x.to_path_buf()) {
						tmp.push((FlacaEncoder::Mozjpeg, y));
					}
				}

				// jpegoptim.
				if let Some(x) = opts.use_jpegoptim {
					if let Some(y) = get_file_canonical(x.to_path_buf()) {
						tmp.push((FlacaEncoder::Jpegoptim, y));
					}
				}

				tmp
			},
			FlacaImageType::Png => {
				let mut tmp = Vec::new();

				// pngout.
				if let Some(x) = opts.use_pngout {
					if let Some(y) = get_file_canonical(x.to_path_buf()) {
						tmp.push((FlacaEncoder::Pngout, y));
					}
				}

				// oxipng.
				if let Some(x) = opts.use_oxipng {
					if let Some(y) = get_file_canonical(x.to_path_buf()) {
						tmp.push((FlacaEncoder::Oxipng, y));
					}
				}

				// Zopflipng.
				if let Some(x) = opts.use_zopflipng {
					if let Some(y) = get_file_canonical(x.to_path_buf()) {
						tmp.push((FlacaEncoder::Zopflipng, y));
					}
				}

				tmp
			},
		};

		if out.len() > 0 {
			return Some(out);
		}

		None
	}
}

/// File.
#[derive(Debug, Clone)]
struct FlacaFile {
	/// Path to image file.
	path: Option<std::path::PathBuf>,
}

impl std::fmt::Display for FlacaFile {
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
		if self.path.is_some() {
			if let Some(ref x) = self.path {
				if let Some(y) = get_file_canonical(x.to_path_buf()) {
					return write!(
						f,
						"{}",
						y
					);
				}
			}
		}

		write!(
			f,
			"{}",
			""
		)
	}
}

impl Default for FlacaFile {
	fn default() -> FlacaFile {
		FlacaFile {
			path: None,
		}
	}
}

impl FlacaFile {
	/// FlacaFile from path.
	fn from(path: std::path::PathBuf) -> FlacaFile {
		if path.is_file() {
			return FlacaFile { path: Some(path) };
		}

		FlacaFile::default()
	}

	/// Compress image using available encoders.
	fn compress(self, opts: FlacaSettings, output: Option<&indicatif::ProgressBar>) -> Result<FlacaResult, FlacaError> {
		// The file has to exist.
		if self.path.is_none() {
			FlacaError::FileInvalid.warn(None);
			return Err(FlacaError::FileInvalid);
		}

		// Make sure we can work on this file.
		let path = self.to_owned().path.unwrap().to_path_buf();
		let ext = get_file_extension(path.to_path_buf());
		if ! path.is_file() || ext.is_none() || ext == opts.skip {
			FlacaError::FileInvalid.warn(get_file_canonical(path.to_owned().to_path_buf()));
			return Err(FlacaError::FileInvalid);
		}

		// Pull encoders.
		let encoders = ext
			.unwrap()
			.encoders(opts.to_owned())
			.unwrap_or(Vec::new());

		// Grab original permissions so we know how to set copies.
		let perms = get_file_perms(path.to_path_buf());
		let owner = get_file_owner(path.to_path_buf());

		// Start a result.
		let mut out = FlacaResult::start(
			path.to_owned().to_path_buf(),
			encoders
				.iter()
				.map(|x| {
					let (a, _) = x;
					a.to_owned()
				})
				.collect()
		)?;

		// Loop our encoders!
		for (a, b) in encoders {
			// Just in case the file vanished.
			if ! path.is_file() {
				FlacaError::FileInvalid.warn(get_file_canonical(path.to_owned().to_path_buf()));
				return Err(FlacaError::FileInvalid);
			}

			if true == opts.debug {
				FlacaError::Debug.debug(Some(format!(
					"Running {} at <{}>",
					a,
					b,
				)), output);
			}

			// Create a working copy.
			let working1 = self.to_owned().working(true);
			if working1.is_none() {
				FlacaError::CommandFailed.warn(Some("Missing working copy.".to_string()));
				continue;
			}

			// Some programs need a second, empty path to write to.
			let mut working2: Option<String> = None;

			// Set up the command.
			let mut com = std::process::Command::new(b);
			match a {
				FlacaEncoder::Jpegoptim => {
					com.arg("-q");
					com.arg("-f");
					com.arg("--strip-all");
					com.arg("--all-progressive");
					com.arg(working1.to_owned().unwrap());
				}
				FlacaEncoder::Mozjpeg => {
					working2 = Some(format!("{}.bak", working1.to_owned().unwrap()));
					com.arg("-copy");
					com.arg("none");
					com.arg("-optimize");
					com.arg("-progressive");
					com.arg("-outfile");
					com.arg(working2.to_owned().unwrap());
					com.arg(working1.to_owned().unwrap());
				}
				FlacaEncoder::Oxipng => {
					com.arg("-s");
					com.arg("-q");
					com.arg("--fix");
					com.arg("-o");
					com.arg("6");
					com.arg("-i");
					com.arg("0");
					com.arg(working1.to_owned().unwrap());
				}
				FlacaEncoder::Pngout => {
					com.arg(working1.to_owned().unwrap());
					com.arg("-q");
				}
				FlacaEncoder::Zopflipng => {
					working2 = Some(format!("{}.bak", working1.to_owned().unwrap()));
					com.arg("-m");
					com.arg(working1.to_owned().unwrap());
					com.arg(working2.to_owned().unwrap());
				}
			}

			// Note the starting size.
			let size_old: u64 = get_file_size(std::path::PathBuf::from(working1.to_owned().unwrap()))
				.unwrap_or(0);

			// Run it.
			if let Err(_) = com
				.stdout(std::process::Stdio::piped())
				.stderr(std::process::Stdio::piped())
				.output() {
				FlacaError::CommandFailed.warn(Some(
					format!(
						"Compress {} with {}.",
						get_file_canonical(path.to_owned().to_path_buf()).unwrap_or("MISSING".to_string()),
						a,
					)
				));

				continue;
			}

			// Move working2 to working1 if it exists.
			if working2.is_some() {
				if let Err(_) = copy_file(
					std::path::PathBuf::from(working2.to_owned().unwrap()),
					std::path::PathBuf::from(working1.to_owned().unwrap()),
					perms.to_owned(),
					owner.to_owned(),
				) {
					FlacaError::CommandFailed.warn(Some(
						format!(
							"Compress {} with {}.",
							get_file_canonical(path.to_owned().to_path_buf()).unwrap_or("MISSING".to_string()),
							a,
						)
					));

					continue;
				}

				if let Err(_) = std::fs::remove_file(std::path::PathBuf::from(working2.to_owned().unwrap())) {
					FlacaError::TmpFailed.warn(None);
					let _noop: bool;
				}
			}

			// Recheck the size in case something changed.
			let size_new: u64 = get_file_size(std::path::PathBuf::from(working1.to_owned().unwrap())).unwrap_or(0);

			if true == opts.debug && size_new > 0 && size_old > 0 {
				FlacaError::Debug.debug(Some(format!(
					"{} / {}",
					indicatif::HumanBytes(size_old),
					match size_new.cmp(&size_old) {
						std::cmp::Ordering::Less => ansi_term::Colour::Green.bold().paint(indicatif::HumanBytes(size_new).to_string()),
						std::cmp::Ordering::Equal => ansi_term::Colour::Yellow.bold().paint(indicatif::HumanBytes(size_new).to_string()),
						std::cmp::Ordering::Greater => ansi_term::Colour::Red.bold().paint(indicatif::HumanBytes(size_new).to_string()),
					}
				)), output);
			}

			// Replace the original image, if applicable.
			if !opts.pretend && size_new > 0 && size_old > 0 && size_new < size_old {
				if let Err(_) = copy_file(
					std::path::PathBuf::from(working1.to_owned().unwrap()),
					std::path::PathBuf::from(path.to_owned().to_path_buf()),
					perms.to_owned(),
					owner.to_owned(),
				) {
					FlacaError::CommandFailed.warn(Some(
						format!(
							"Compress {}.",
							get_file_canonical(path.to_owned().to_path_buf()).unwrap_or("MISSING".to_string()),
						)
					));
				}
			}

			// Clean up the working file.
			if let Err(_) = std::fs::remove_file(std::path::PathBuf::from(working1.to_owned().unwrap())) {
				FlacaError::TmpFailed.warn(None);
				let _noop: bool;
			}
		}

		out.finish()?;

		// Log it?
		if opts.log.is_some() {
			if true == opts.debug {
				FlacaError::Debug.debug(Some(format!(
					"Logging result to <{}>",
					get_file_name(opts.to_owned().log.unwrap()).unwrap_or("UNKNOWN".to_string()),
				)), output);
			}

			out.log(opts.log.unwrap());
		}

		Ok(out)
	}

	/// Wrapper to obtain canonical path.
	fn canonical(self) -> Option<String> {
		match self.path {
			Some(x) => get_file_canonical(x.to_path_buf()),
			None => None,
		}
	}

	/// Wrapper to obtain extension.
	fn extension(self) -> Option<FlacaImageType> {
		match self.path {
			Some(x) => get_file_extension(x.to_path_buf()),
			None => None,
		}
	}

	/// Wrapper to obtain file name.
	fn name(self) -> Option<String> {
		match self.path {
			Some(x) => get_file_name(x.to_path_buf()),
			None => None,
		}
	}

	/// Generate (Unique) Working Copy.
	fn working(self, copy: bool) -> Option<String> {
		// First we need a unique file.
		if let Some(ext) = self.to_owned().extension() {
			if let Some(name) = self.to_owned().name() {
				let mut num: u64 = 0;
				let dir = std::path::PathBuf::from(std::env::temp_dir());

				// This should be a directory.
				if dir.is_dir() {
					if let Some(base) = get_file_canonical(dir) {
						// Guess at a likely unique name.
						let mut out_name: String = format!(
							"{}/{}.__flaca{}.{}",
							base,
							name,
							num.to_string(),
							ext
						);

						// Repeat until we have something unique.
						while std::path::Path::new(&out_name).exists() {
							num += 1;

							out_name = format!(
								"{}/{}.__flaca{}.{}",
								base,
								name,
								num.to_string(),
								ext
							);
						}

						// Copy?
						if true == copy {
							if let Err(_) = copy_file(self.path.unwrap().to_path_buf(), std::path::PathBuf::from(&out_name), None, None) {
								return None;
							}
						}

						// Done!
						return Some(out_name);
					}
				}
			}
		}

		None
	}
}

#[derive(Debug, Clone)]
struct FlacaResult {
	/// Source path.
	path: Option<std::path::PathBuf>,
	/// Encoders being used.
	encoders: Option<Vec<FlacaEncoder>>,
	/// Start time.
	start_time: Option<std::time::SystemTime>,
	/// End time.
	end_time: Option<std::time::SystemTime>,
	/// Start size.
	start_size: Option<u64>,
	/// End size.
	end_size: Option<u64>,
	/// The elapsed time.
	elapsed: Option<std::time::Duration>,
	/// The total bytes saved.
	saved: Option<u64>,
}

impl Default for FlacaResult {
	fn default() -> FlacaResult {
		FlacaResult {
			path: None,
			encoders: None,
			start_time: Some(std::time::SystemTime::now()),
			end_time: None,
			start_size: None,
			end_size: None,
			elapsed: None,
			saved: None,
		}
	}
}

impl FlacaResult {
	/// Initialize a result object.
	fn start(path: std::path::PathBuf, encoders: Vec<FlacaEncoder>) -> Result<FlacaResult, FlacaError> {
		if ! path.is_file() {
			FlacaError::FileInvalid.warn(Some(
				format!(
					"Compress {}.",
					"MISSING".to_string(),
				)
			));
			return Err(FlacaError::FileInvalid);
		}

		let size = get_file_size(path.to_path_buf());

		Ok(FlacaResult {
			path: Some(path),
			start_size: size,
			encoders: Some(encoders),
			..FlacaResult::default()
		})
	}

	/// Wrap up and report what was done.
	fn finish(&mut self) -> Result<(), FlacaError> {
		if self.path.is_none() {
			FlacaError::FileInvalid.warn(Some(
				format!(
					"Compress {}.",
					"MISSING".to_string(),
				)
			));
			return Err(FlacaError::FileInvalid);
		}

		let path = self.to_owned().path.unwrap();

		// Finish time.
		self.end_time = Some(std::time::SystemTime::now());
		if self.start_time.is_some() {
			if let Ok(x) = self.end_time.unwrap().duration_since(self.start_time.unwrap()) {
				self.elapsed = Some(x);
			}
		}

		// Check the size.
		self.end_size = get_file_size(path.to_path_buf());
		if self.start_size.is_some() && self.end_size != self.start_size {
			self.saved = Some(self.start_size.unwrap() - self.end_size.unwrap());
		}

		// Done!
		Ok(())
	}

	/// Log results to file.
	fn log(&self, log: std::path::PathBuf) {
		// No path, nothing to do.
		if self.path.is_none() || self.elapsed.is_none() {
			return;
		}

		// Put together a human-readable status string.
		let status: String = match self.saved {
			Some(x) => format!(
				"Saved {} bytes in {} seconds.",
				x,
				self.elapsed.unwrap().as_secs(),
			),
			None => "No change.".to_string(),
		};

		let mut file = std::fs::OpenOptions::new()
			.write(true)
			.append(true)
			.create(true)
			.open(log)
			.unwrap();

		if let Err(_) = writeln!(
			file,
			"{} \"{}\" {} {} {}",
			get_local_now().to_rfc3339(),
			get_file_canonical(self.to_owned().path.unwrap()).unwrap(),
			self.start_size.unwrap_or(0),
			self.end_size.unwrap_or(0),
			status,
		) {
			FlacaError::LogFailed.warn(get_file_canonical(self.to_owned().path.unwrap()));
			return;
		}
	}
}

/// Runtime settings.
#[derive(Debug, Clone)]
struct FlacaSettings {
	/// Debug messages.
	debug: bool,
	/// Dry run; do not override source files.
	pretend: bool,
	/// Suppress STDOUT.
	quiet: bool,

	/// Path to log.
	log: Option<std::path::PathBuf>,

	/// Ignore files younger than X.
	min_age: Option<u64>,
	/// Ignore files older than X.
	max_age: Option<u64>,
	/// Ignore files smaller than X.
	min_size: Option<u64>,
	/// Ignore files bigger than X.
	max_size: Option<u64>,

	/// Skip a format.
	skip: Option<FlacaImageType>,

	/// Alternate jpegoptim binary path.
	use_jpegoptim: Option<std::path::PathBuf>,
	/// Alternate MozJPEG binary path.
	use_mozjpeg: Option<std::path::PathBuf>,
	/// Alternate oxipng binary path.
	use_oxipng: Option<std::path::PathBuf>,
	/// Alternate pngout binary path.
	use_pngout: Option<std::path::PathBuf>,
	/// Alternate zopflipng binary path.
	use_zopflipng: Option<std::path::PathBuf>,

	/// List of images to process; this is built automatically from other options.
	images: Option<Vec<FlacaFile>>,
}

impl Default for FlacaSettings {
	fn default() -> FlacaSettings {
		FlacaSettings {
			debug: false,
			pretend: false,
			quiet: false,
			log: None,
			min_age: None,
			max_age: None,
			min_size: None,
			max_size: None,
			skip: None,
			use_jpegoptim: FlacaEncoder::Jpegoptim.bin_path(None),
			use_mozjpeg: FlacaEncoder::Mozjpeg.bin_path(None),
			use_oxipng: FlacaEncoder::Oxipng.bin_path(None),
			use_pngout: FlacaEncoder::Pngout.bin_path(None),
			use_zopflipng: FlacaEncoder::Zopflipng.bin_path(None),
			images: None,
		}
	}
}

impl FlacaSettings {
	/// Parse args into FlacaSettings struct.
	fn from(args: &clap::ArgMatches) -> FlacaSettings {
		let debug: bool = args.is_present("debug");

		// Most of this can be built straight away.
		let mut out: FlacaSettings = FlacaSettings {
			debug: debug,
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
				Some(x) => FlacaImageType::from(std::path::PathBuf::from(x)),
				None => None,
			},
			use_jpegoptim: FlacaEncoder::Jpegoptim.bin_path(
				match args.value_of("use_jpegoptim") {
					Some(x) => Some(std::path::PathBuf::from(x)),
					None => None,
				}
			),
			use_mozjpeg: FlacaEncoder::Mozjpeg.bin_path(
				match args.value_of("use_mozjpeg") {
					Some(x) => Some(std::path::PathBuf::from(x)),
					None => None,
				}
			),
			use_oxipng: FlacaEncoder::Oxipng.bin_path(
				match args.value_of("use_oxipng") {
					Some(x) => Some(std::path::PathBuf::from(x)),
					None => None,
				}
			),
			use_pngout: FlacaEncoder::Pngout.bin_path(
				match args.value_of("use_pngout") {
					Some(x) => Some(std::path::PathBuf::from(x)),
					None => None,
				}
			),
			use_zopflipng: FlacaEncoder::Zopflipng.bin_path(
				match args.value_of("use_zopflipng") {
					Some(x) => Some(std::path::PathBuf::from(x)),
					None => None,
				}
			),
			..Self::default()
		};

		// Depending on what is installed on the system, we may not be
		// able to process certain image types.
		if Some(FlacaImageType::Jpg) != out.skip && out.use_jpegoptim.is_none() && out.use_mozjpeg.is_none() {
			// No skip is set, so we can skip JPEGs.
			if out.skip.is_none() {
				out.skip = Some(FlacaImageType::Jpg);
				FlacaError::NoDepsJpg.warn(None);
			}
			// Nothing to process means an error.
			else if Some(FlacaImageType::Png) == out.skip {
				FlacaError::NoDepsJpg.error(None);
			}
		}
		else if Some(FlacaImageType::Png) != out.skip && out.use_oxipng.is_none() && out.use_pngout.is_none() && out.use_zopflipng.is_none() {
			// No skip is set, so we can skip JPEGs.
			if out.skip.is_none() {
				out.skip = Some(FlacaImageType::Png);
				FlacaError::NoDepsPng.warn(None);
			}
			// Nothing to process means an error.
			else if Some(FlacaImageType::Jpg) == out.skip {
				FlacaError::NoDepsPng.error(None);
			}
		}

		// Now we need to see if any images map.
		let mut images = out.parse_images(
			args
				.values_of("INPUT")
				.unwrap()
				.map(std::path::PathBuf::from)
				.collect(),
		);

		// Abort if there were no images.
		if images.len() < 1 {
			FlacaError::NoFiles.error(None);
		}

		// Otherwise sort, dedup, and convert!
		images.sort();
		images.dedup();
		out.images = Some(
			images
				.iter()
				.map(|x| { FlacaFile::from(std::path::PathBuf::from(x)) })
				.collect()
			);

		// Done!
		out
	}

	/// Recursive callback to find applicable images.
	fn parse_images(&self, files: Vec<std::path::PathBuf>) -> Vec<String> {
		let mut out = Vec::new();

		for image in files {
			// Recurse directories.
			if image.is_dir() {
				let files = image
					.read_dir()
					.unwrap()
					.map(|x| x.unwrap().path().to_owned())
					.collect();
				out.extend(self.parse_images(files));
			}
			// Just a regular old file.
			else if image.is_file() {
				// Should be an expandable path.
				if let Some(path) = get_file_canonical(image.to_path_buf()) {
					// Check extension first.
					if let Some(ext) = FlacaImageType::from(image.to_path_buf()) {
						// Skipping this type.
						if self.skip == Some(ext) {
							continue;
						}

						// Check file size.
						if self.min_size.is_some() || self.max_size.is_some() {
							if let Some(size) = get_file_size(image.to_path_buf()) {
								if (self.min_size.is_some() && size < self.min_size.unwrap()) || (self.max_size.is_some() && size > self.max_size.unwrap()) {
									continue;
								}
							} else {
								continue;
							}
						}

						// Check file time.
						if self.min_age.is_some() || self.max_age.is_some() {
							if let Some(age) = get_file_modified_since(image.to_path_buf()) {
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

	/// Count up found images.
	fn total_images(&self) -> Option<u64> {
		if let Some(images) = self.to_owned().images {
			return Some(images.len() as u64);
		}

		None
	}

	/// Count up byte size of found images.
	fn total_size(&self) -> Option<u64> {
		if let Some(images) = self.to_owned().images {
			let mut size: u64 = 0;

			for i in images {
				if let Some(x) = i.path {
					if let Some(y) = get_file_size(x.to_path_buf()) {
						size += y;
					}
				}
			}

			return Some(size);
		}

		None
	}
}



// ---------------------------------------------------------------------
// File Stat Helpers
// ---------------------------------------------------------------------

/// Get a file's canonical path.
fn get_file_canonical(path: std::path::PathBuf) -> Option<String> {
	match path.canonicalize() {
		Ok(x) => Some(format!("{}", x.display())),
		Err(_) => None,
	}
}

/// Get a file's extension.
fn get_file_extension(path: std::path::PathBuf) -> Option<FlacaImageType> {
	FlacaImageType::from(path)
}

/// Get a file's modification time.
fn get_file_modified(path: std::path::PathBuf) -> Option<std::time::SystemTime> {
	if let Ok(x) = path.metadata() {
		if let Ok(y) = x.modified() {
			return Some(y);
		}
	}

	None
}

/// Get a file's relative (to now) modification time in seconds.
fn get_file_modified_since(path: std::path::PathBuf) -> Option<u64> {
	if let Some(x) = get_file_modified(path) {
		let now = std::time::SystemTime::now();
		if let Ok(y) = now.duration_since(x) {
			return Some(y.as_secs());
		}
	}

	None
}

/// Get a file's name.
fn get_file_name(path: std::path::PathBuf) -> Option<String> {
	if let Some(x) = path.file_name() {
		if let Some(y) = std::ffi::OsStr::to_str(x) {
			return Some(y.to_string());
		}
	}

	None
}

/// Get a file's User ID and Group ID.
fn get_file_owner(path: std::path::PathBuf) -> Option<(nix::unistd::Uid, nix::unistd::Gid)> {
	match path.metadata() {
		Ok(x) => Some((
			nix::unistd::Uid::from_raw(x.uid()),
			nix::unistd::Gid::from_raw(x.gid()),
		)),
		Err(_) => None,
	}
}

/// Get a file's permissions.
fn get_file_perms(path: std::path::PathBuf) -> Option<std::fs::Permissions> {
	match path.metadata() {
		Ok(x) => Some(x.permissions()),
		Err(_) => None,
	}
}

/// Get a file's disk size.
fn get_file_size(path: std::path::PathBuf) -> Option<u64> {
	match path.metadata() {
		Ok(x) => Some(x.len()),
		Err(_) => None,
	}
}



// ---------------------------------------------------------------------
// File IO Helpers
// ---------------------------------------------------------------------

/// Copy a file.
fn copy_file(
	from: std::path::PathBuf,
	to: std::path::PathBuf,
	perms: Option<std::fs::Permissions>,
	owner: Option<(nix::unistd::Uid, nix::unistd::Gid)>
) -> Result<(), FlacaError> {
	// No file, no copy.
	if ! from.is_file() {
		FlacaError::CommandFailed.warn(Some(
			format!(
				"Copy {} to {}.",
				"MISSING".to_string(),
				get_file_canonical(to.to_path_buf()).unwrap_or("MISSING".to_string()),
			)
		));

		return Err(FlacaError::FileInvalid);
	}

	// We might have to delete the destination.
	if to.exists() {
		if let Err(_) = std::fs::remove_file(to.to_path_buf()) {
			FlacaError::CommandFailed.warn(Some(
				format!(
					"Copy {} to {}.",
					get_file_canonical(from.to_owned().to_path_buf()).unwrap_or("MISSING".to_string()),
					get_file_canonical(to.to_path_buf()).unwrap_or("MISSING".to_string()),
				)
			));

			return Err(FlacaError::CommandFailed);
		}
	}

	// Try to copy.
	if let Err(_) = std::fs::copy(from.to_path_buf(), to.to_path_buf()) {
		FlacaError::CommandFailed.warn(Some(
			format!(
				"Copy {} to {}.",
				get_file_canonical(from.to_owned().to_path_buf()).unwrap_or("MISSING".to_string()),
				get_file_canonical(to.to_path_buf()).unwrap_or("MISSING".to_string()),
			)
		));

		return Err(FlacaError::CommandFailed);
	}

	// Set permissions?
	if let Some(x) = perms {
		if let Err(_) = std::fs::set_permissions(to.as_path(), x) {
			FlacaError::PermsFailed.warn(get_file_canonical(from.to_owned().to_path_buf()));
			return Ok(())
		}
	}

	// Set owner?
	if let Some((uid, gid)) = owner {
		if let Err(_) = nix::unistd::chown(to.as_path(), Some(uid), Some(gid)) {
			FlacaError::PermsFailed.warn(get_file_canonical(from.to_owned().to_path_buf()));
			return Ok(())
		}
	}

	Ok(())
}



// ---------------------------------------------------------------------
// Dates and Time
// ---------------------------------------------------------------------

/// Get current, local time.
fn get_local_now() -> chrono::DateTime<chrono::Local> {
	let start = std::time::SystemTime::now();
	let start_since = start.duration_since(std::time::UNIX_EPOCH).expect("Time is meaningless.");

	chrono::Local.timestamp(start_since.as_secs() as i64, 0)
}



// ---------------------------------------------------------------------
// Arg Validation
// ---------------------------------------------------------------------

/// Args validation for min/max age and size.
fn validate_args_min_max(x: String) -> Result<(), String> {
    match x.parse::<u64>() {
		Ok(y) => {
			if y > 0 {
				Ok(())
			} else {
				Err(FlacaError::UsizeMinMax.to_string(None))
			}
		}
		Err(_) => Err(FlacaError::UsizeMinMax.to_string(None))
	}
}

/// Args validation for log path.
fn validate_args_log(x: String) -> Result<(), String> {
    let path = std::path::PathBuf::from(x);

	// This can't be a directory. Haha.
	if path.is_dir() {
		return Err(FlacaError::FileInvalid.to_string(get_file_canonical(path)));
	}

	return Ok(());
}



// ---------------------------------------------------------------------
// Binary!
// ---------------------------------------------------------------------

fn main() {
	// Set up runtime arguments.
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
			.help("Log operations to this location.")
			.takes_value(true)
			.validator(validate_args_log)
			.value_name("PATH")
		)
		.arg(clap::Arg::with_name("min_age")
			.long("min_age")
			.alias("min-age")
			.help("Ignore files younger than this.")
			.takes_value(true)
			.validator(validate_args_min_max)
			.value_name("MINUTES")
		)
		.arg(clap::Arg::with_name("max_age")
			.long("max_age")
			.alias("max-age")
			.help("Ignore files older than this.")
			.takes_value(true)
			.validator(validate_args_min_max)
			.value_name("MINUTES")
		)
		.arg(clap::Arg::with_name("min_size")
			.long("min_size")
			.alias("min-size")
			.help("Ignore files smaller than this.")
			.takes_value(true)
			.validator(validate_args_min_max)
			.value_name("BYTES")
		)
		.arg(clap::Arg::with_name("max_size")
			.long("max_size")
			.alias("max-size")
			.help("Ignore files larger than this.")
			.takes_value(true)
			.validator(validate_args_min_max)
			.value_name("BYTES")
		)
		.arg(clap::Arg::with_name("quiet")
			.short("q")
			.long("quiet")
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
		.arg(clap::Arg::with_name("use_jpegoptim")
			.long("use_jpegoptim")
			.alias("jpegoptim")
			.alias("use-jpegoptim")
			.help("Alternate binary path for jpegoptim.")
			.takes_value(true)
			.value_name("BIN")
		)
		.arg(clap::Arg::with_name("use_mozjpeg")
			.long("use_mozjpeg")
			.alias("jpegtran")
			.alias("mozjpeg")
			.alias("use-jpegtran")
			.alias("use-mozjpeg")
			.alias("use_jpegtran")
			.help("Alternate binary path for MozJPEG.")
			.takes_value(true)
			.value_name("BIN")
		)
		.arg(clap::Arg::with_name("use_oxipng")
			.long("use_oxipng")
			.alias("oxipng")
			.alias("use-oxipng")
			.help("Alternate binary path for oxipng.")
			.takes_value(true)
			.value_name("BIN")
		)
		.arg(clap::Arg::with_name("use_pngout")
			.long("use_pngout")
			.alias("pngout")
			.alias("use-pngout")
			.help("Alternate binary path for pngout.")
			.takes_value(true)
			.value_name("BIN")
		)
		.arg(clap::Arg::with_name("use_zopflipng")
			.long("use_zopflipng")
			.alias("use-zopflipng")
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
    jpegoptim <https://github.com/tjko/jpegoptim>
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    oxipng    <https://github.com/shssoichiro/oxipng>
    pngout    <http://advsys.net/ken/utils.htm>
    zopflipng <https://github.com/google/zopfli>
		")
		.get_matches();

	let opts = FlacaSettings::from(&args);

	// Print header.
	header(opts.to_owned());

	// Quiet version.
	if opts.quiet {
		process_images_quiet(opts);
	}
	// Pretty version.
	else {
		if true == opts.debug {
			FlacaError::Debug.debug(Some("Verbose output coming…".to_string()), None);

			if opts.pretend {
				FlacaError::Debug.debug(Some("Pretend mode; no files will be overwritten.".to_string()), None);
			}

			if opts.skip.is_some() {
				FlacaError::Debug.debug(Some(format!(
					"Skipping: {} images.",
					opts.skip.unwrap(),
				)), None);
			}

			if opts.min_size.is_some() || opts.max_size.is_some() {
				FlacaError::Debug.debug(Some(format!(
					"Looking for images between {} and {} bytes.",
					opts.min_size.unwrap_or(0),
					match opts.max_size {
						Some(x) => x.to_string(),
						None => "∞".to_string(),
					},
				)), None);
			}

			if opts.min_age.is_some() || opts.max_age.is_some() {
				FlacaError::Debug.debug(Some(format!(
					"Looking for images between {} and {} minutes old.",
					opts.min_age.unwrap_or(0) / 60,
					match opts.max_age {
						Some(x) => (x / 60).to_string(),
						None => "∞".to_string(),
					},
				)), None);
			}
		}

		process_images(opts);
	}
}

/// Print CLI header.
fn header(opts: FlacaSettings) {
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
		opts.total_images().unwrap_or(0),
		ansi_term::Colour::Blue.bold().paint("Space:"),
		indicatif::HumanBytes(opts.total_size().unwrap_or(0)),
		"Ready, Set, Goat!",
	);
}

/// Process Images
fn process_images(opts: FlacaSettings) {
	// Set up progress bar.
	let pb: indicatif::ProgressBar = indicatif::ProgressBar::new(opts.total_images().unwrap_or(0));
	pb.set_style(
		indicatif::ProgressStyle::default_bar()
			.template("[{elapsed_precise}] [{bar:40.cyan/blue}]  {pos:.cyan.bold}/{len:.blue.bold}  {percent:.bold}%  {msg}")
			.progress_chars("##-")
	);

	// Beginnings.
	let start_time = std::time::SystemTime::now();
	let start_size = opts.total_size().unwrap_or(0);
	let mut total_saved: u64 = 0;

	// Loop the images!
	for i in opts.to_owned().images.unwrap() {
		// File went away?
		if i.path.to_owned().is_none() || ! i.path.to_owned().unwrap().is_file() {
			FlacaError::FileInvalid.warn(None);
			continue;
		}

		// Announce we've started.
		pb.println(format!(
			"[{}] {}",
			ansi_term::Colour::Purple.bold().paint(get_local_now().to_rfc3339()),
			i.to_owned().canonical().unwrap(),
		));

		let result = i.compress(opts.to_owned(), Some(&pb));

		// Bump progress.
		pb.inc(1);

		if result.is_err() {
			continue;
		}

		// Print progress.
		if let Some(x) = result.unwrap().saved {
			if x > 0 {
				total_saved += x;
				pb.set_message(&format!(
					"{}",
					ansi_term::Colour::Green.bold().paint(format!(
						"-{}", indicatif::HumanBytes(total_saved)
					)),
				));
			}
		}
	}

	// Kill the progress bar.
	pb.finish_and_clear();

	// Endings.
	let end_time = std::time::SystemTime::now();
	let end_size = opts.total_size().unwrap_or(0);
	let end_elapsed = end_time.duration_since(start_time).expect(&FlacaError::TimeFailed.to_string(None));
	let end_saved = start_size - end_size;

	println!(
		"{} {}",
		ansi_term::Colour::Green.bold().paint("Finished:"),
		indicatif::HumanDuration(end_elapsed),
	);

	// We were able to save some space.
	if end_saved > 0 {
		println!(
			"{} {}",
			ansi_term::Colour::Green.bold().paint("Saved:"),
			indicatif::HumanBytes(end_saved),
		);
	}
	// Nothing changed.
	else {
		println!(
			"{} 0B",
			ansi_term::Colour::Yellow.bold().paint("Saved:"),
		);
	}
}

/// Process Images (Quiet)
fn process_images_quiet(opts: FlacaSettings) {
	// Loop the images!
	for i in opts.to_owned().images.unwrap() {
		if let Err(_) = i.compress(opts.to_owned(), None) {
			continue;
		}
	}
}
