/*!
# Flaca

Brute-force, lossless JPEG and PNG compression.
*/

#![warn(missing_docs)]
#![warn(trivial_casts)]
#![warn(trivial_numeric_casts)]
#![warn(unused_import_braces)]

#![deny(missing_copy_implementations)]
#![deny(missing_debug_implementations)]

#![warn(clippy::filetype_is_file)]
#![warn(clippy::integer_division)]
#![warn(clippy::needless_borrow)]
#![warn(clippy::pedantic)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

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

	// What path are we dealing with?
	let walk: Witch = if opts.is_present("list") {
		Witch::from_file(
			opts.value_of("list").unwrap_or(""),
			Some(r"(?i).+\.(jpe?g|png)$".to_string())
		)
	}
	else {
		Witch::new(
			&opts.values_of("path")
				.unwrap()
				.collect::<Vec<&str>>(),
			Some(r"(?i).+\.(jpe?g|png)$".to_string())
		)
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
		walk.process(|x| {
			let _ = x.flaca_encode().is_ok();
		});
	}

	Ok(())
}
