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
};
use fyi_witch::Witch;



fn main() -> Result<()> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// Check dependencies before doing anything else.
	flaca_core::check_dependencies();

	let walk: Witch = match opts.is_present("list") {
		false => Witch::new(
			&opts.values_of("path")
				.unwrap()
				.collect::<Vec<&str>>(),
			Some(r"(?i).+\.(jpe?g|png)$".to_string())
		),
		true => Witch::from_file(
			opts.value_of("list").unwrap_or(""),
			Some(r"(?i).+\.(jpe?g|png)$".to_string())
		),
	};

	if walk.is_empty() {
		return Err(Error::new("No images were found."));
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
