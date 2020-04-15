/*!
# Flaca

Brute-force, lossless JPEG and PNG compression.
*/

extern crate clap;
extern crate flaca_core;
extern crate fyi_core;

mod menu;

use clap::ArgMatches;
use flaca_core::image::ImagePath;
use fyi_core::{
	Error,
	Result,
	Witch
};
use std::path::PathBuf;



fn main() -> Result<()> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// Check dependencies before doing anything else.
	flaca_core::check_dependencies();

	// What path are we dealing with?
	let walk: Witch = match opts.is_present("list") {
		false => {
			let paths: Vec<PathBuf> = opts.values_of("path").unwrap()
				.into_iter()
				.map(|x| PathBuf::from(x))
				.collect();

			Witch::new(
				&paths,
				Some(r"(?i).+\.(jpe?g|png)$".to_string())
			)
		},
		true => {
			let path = PathBuf::from(opts.value_of("list").unwrap_or(""));
			Witch::from_file(
				&path,
				Some(r"(?i).+\.(jpe?g|png)$".to_string())
			)
		},
	};

	if walk.is_empty() {
		return Err(Error::NoPaths("images".into()));
	}

	// With progress.
	if opts.is_present("progress") {
		walk.progress_crunch("Flaca", |x| {
			let _ = x.flaca_encode().is_ok();
		});
	}
	// Without progress.
	else {
		walk.process(|ref x| {
			let _ = x.flaca_encode().is_ok();
		});
	}

	Ok(())
}
