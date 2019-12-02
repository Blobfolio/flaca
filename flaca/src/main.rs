/*!
# Flaca CLI

Flaca losslessly compresses JPEG and PNG images *as much as possible*.
It achieves this through guided brute-force, passing images through a
series of independent optimizers — some of which are better at handling
certain types of content than others.

These third-party optimizers are not bundled with Flaca and must be
present on the host system to do their thing. Flaca will skip optimizers
it can't find, but for best results, it is recommended systems install
everything in the lists below.

JPEG images are sent to (in order):
* [MozJPEG](https://github.com/mozilla/mozjpeg)
* [Jpegoptim](https://github.com/tjko/jpegoptim)

PNG images are sent to (in order):
* [PNGOUT](http://advsys.net/ken/utils.htm)
* [Oxipng](https://github.com/shssoichiro/oxipng)
* [Zopflipng](https://github.com/google/zopfli)

For a list of options, run `flaca --help`.
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]



extern crate ansi_escapes;
extern crate ansi_term;
extern crate clap;
extern crate crossbeam_channel;
extern crate flaca_core;
extern crate rayon;
extern crate strip_ansi_escapes;
extern crate term_size;



mod display;

use crate::display::Display;
use flaca_core::{Core, Config, FlacaError, Format, ImageApp, Reporter};
use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};



fn main() -> Result<(), String> {
	let (core, display, paths) = init_cli();
	let arc_display = Arc::new(Mutex::new(display));
	let arc_config = core.config();
	let arc_reporter = core.reporter();

	// No paths?
	if paths.is_empty() {
		Display::arc_die(arc_display.clone(), FlacaError::NoImages);
	}
	// Double-dipping somehow?
	else if Reporter::arc_running(arc_reporter.clone()) {
		Display::arc_die(arc_display.clone(), FlacaError::AlreadyRunning);
	}

	// Push the display into its own thread.
	Display::arc_reset(arc_display.clone());
	let arc_display2 = arc_display.clone();
	let arc_config2 = arc_config.clone();

	let display_handle = || Display::arc_watch(arc_display2.clone(), arc_config2.clone());
	let core_handle = || core.run(&paths);
	let (_, core_result) = rayon::join(display_handle, core_handle);

	// In the meantime, let's process our images!
	if let Err(e) = core_result {
		Display::arc_die(arc_display.clone(), e);
	}

	Ok(())
}

/// Initialize CLI
///
/// This is really 99% of the application; we're just shoving it here to
/// keep `main()` somewhat readable.
fn init_cli() -> (Core, Display, Vec<PathBuf>) {
	// Initialize Clap first.
	let args: clap::ArgMatches = clap::App::new("Flaca")
		.version(env!("CARGO_PKG_VERSION"))
		.author("Blobfolio, LLC. <hello@blobfolio.com>")
		.about(env!("CARGO_PKG_DESCRIPTION"))
		.arg(clap::Arg::with_name("dry_run")
			.long("dry-run")
			.help("Test compression without updating the original files.")
			.takes_value(false)
		)
		.arg(clap::Arg::with_name("file_list")
			.long("file-list")
			.help("Load image paths (and/or directory paths) from the text file passed as <IMAGE PATH(S)>.")
			.takes_value(false)
		)
		.arg(clap::Arg::with_name("level")
			.long("reporting-level")
			.help("Output verbosity between 0 (quiet) and 4 (debug).")
			.takes_value(true)
			.value_name("LEVEL")
			.validator(validate_cli_level)
		)
		.arg(clap::Arg::with_name("x_jpegoptim")
			.long("jpegoptim")
			.help("Path to Jpegoptim binary.")
			.takes_value(true)
			.value_name("PATH")
			.validator(validate_cli_path)
		)
		.arg(clap::Arg::with_name("x_mozjpeg")
			.long("mozjpeg")
			.help("Path to MozJPEG binary.")
			.takes_value(true)
			.value_name("PATH")
			.validator(validate_cli_path)
		)
		.arg(clap::Arg::with_name("x_oxipng")
			.long("oxipng")
			.help("Path to Oxipng binary.")
			.takes_value(true)
			.value_name("PATH")
			.validator(validate_cli_path)
		)
		.arg(clap::Arg::with_name("x_pngout")
			.long("pngout")
			.help("Path to Pngout binary.")
			.takes_value(true)
			.value_name("PATH")
			.validator(validate_cli_path)
		)
		.arg(clap::Arg::with_name("x_zopflipng")
			.long("zopflipng")
			.help("Path to Zopflipng binary.")
			.takes_value(true)
			.value_name("PATH")
			.validator(validate_cli_path)
		)
		.arg(clap::Arg::with_name("INPUT")
			.index(1)
			.help("Paths to image files and/or directories with image files, or if --file-list is set, a text file containing same.")
			.multiple(true)
			.required(true)
			.value_name("IMAGE PATH(S)")
			.use_delimiter(false)
		)
		.after_help("SUPPORTED OPTIMIZERS:
    Jpegoptim <https://github.com/tjko/jpegoptim>
    MozJPEG   <https://github.com/mozilla/mozjpeg>
    Oxipng    <https://github.com/shssoichiro/oxipng>
    Pngout    <http://advsys.net/ken/utils.htm>
    Zopflipng <https://github.com/google/zopfli>
		")
		.get_matches();

	let config: Config = init_config(&args);
	let display: Display = Display::new(config.reporter());
	let core: Core = Core::new(config);
	let paths: Vec<PathBuf> = init_paths(&args);

	(core, display, paths)
}

/// Initialize `flaca_core::Config`
///
/// The runtime settings build from three distinct levels:
/// * Flaca defaults
/// * Global configuration stored at `/etc/flaca.yml`
/// * Command line arguments
fn init_config(args: &clap::ArgMatches) -> Config {
	// Start with default values stored under `/etc/flaca.yml`.
	let config: Config = Config::load("/etc/flaca.yml");

	// Turn on dry run?
	if args.is_present("dry_run") {
		config.set_dry_run(true);
	}

	// Set the level?
	if let Some(x) = args.value_of("level") {
		if let Ok(x) = x.parse::<u64>() {
			config.set_level(x as usize);
		}
	}

	// Set any app paths?
	if let Some(x) = args.value_of("x_jpegoptim") {
		config.set_jpegoptim(ImageApp::try_jpegoptim(x));
	}
	if let Some(x) = args.value_of("x_mozjpeg") {
		config.set_mozjpeg(ImageApp::try_mozjpeg(x));
	}
	if let Some(x) = args.value_of("x_oxipng") {
		config.set_oxipng(ImageApp::try_oxipng(x));
	}
	if let Some(x) = args.value_of("x_pngout") {
		config.set_pngout(ImageApp::try_pngout(x));
	}
	if let Some(x) = args.value_of("x_zopflipng") {
		config.set_zopflipng(ImageApp::try_zopflipng(x));
	}

	// And we're done!
	config
}

/// Parse Paths From Args.
///
/// Image paths can be stored in a text file — if --file-list is
/// specified — or from the variatic arguments at the end of the
/// command.
///
/// Either way, this method returns a vector of existing paths. Flaca
/// Core will examine the list we produce in more detail to make sure
/// the entries are actually images, etc.
fn init_paths(args: &clap::ArgMatches) -> Vec<PathBuf> {
	// Pull from a text file.
	if args.is_present("file_list") {
		// Pull from a file.
		if let Some(raw) = args.value_of("INPUT") {
			if let Ok(paths) = init_paths_from_file(raw) {
				return paths;
			}
		}
	}
	// Pull from command arguments.
	else if let Some(raw) = args.values_of("INPUT") {
		let raw2: Vec<String> = raw.filter_map(|x| Some(x.to_string())).collect();
		if let Ok(paths) = parse_paths(&raw2) {
			return paths;
		}
	}

	Vec::new()
}

/// Paths From File.
fn init_paths_from_file<P> (path: P) -> Result<Vec<PathBuf>, FlacaError>
where P: AsRef<Path> {
	let input = File::open(path)?;
	let buffered = BufReader::new(input);

	let out: Vec<String> = buffered.lines()
		.filter_map(|x| match x.ok() {
			Some(x) => {
				let x = x.trim();
				match x.is_empty() {
					true => None,
					false => Some(x.to_string()),
				}
			},
			_ => None,
		})
		.collect();

	parse_paths(&out)
}

/// Parse Paths (From String).
fn parse_paths(paths: &Vec<String>) -> Result<Vec<PathBuf>, FlacaError> {
	let out: Vec<PathBuf> = paths.iter()
		.filter_map(|x| {
			if false == x.is_empty() {
				let path = PathBuf::from(x);
				if path.exists() {
					return Some(path);
				}
			}

			None
		})
		.collect();

	match out.is_empty() {
		true => Err(FlacaError::NoImages),
		false => Ok(out)
	}
}

/// Validate CLI Setting: Level
fn validate_cli_level(val: String) -> Result<(), String> {
	if let Ok(x) = val.parse::<u64>() {
		if 4 >= x {
			return Ok(());
		}
	}

	Err("Value must be between 0–4.".to_string())
}

/// Validate CLI Setting: App Path
fn validate_cli_path(val: String) -> Result<(), String> {
	match Format::path::is_executable(&val) {
		true => Ok(()),
		false => Err("Invalid path.".to_string()),
	}
}
