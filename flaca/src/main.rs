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
#![warn(clippy::nursery)]
#![warn(clippy::pedantic)]
#![warn(clippy::perf)]
#![warn(clippy::suboptimal_flops)]
#![warn(clippy::unneeded_field_pattern)]

#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_errors_doc)]

mod menu;

use clap::ArgMatches;
use flaca_core::encode_image;
use fyi_witcher::{
	Witcher,
	Result,
};



fn main() -> Result<()> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// Check dependencies before doing anything else.
	flaca_core::check_dependencies();

	// What path are we dealing with?
	let walk = if opts.is_present("list") {
		Witcher::from_file(
			opts.value_of("list").unwrap_or(""),
			r"(?i).+\.(jpe?g|png)$"
		)
	}
	else {
		Witcher::new(
			&opts.values_of("path")
				.unwrap()
				.collect::<Vec<&str>>(),
			r"(?i).+\.(jpe?g|png)$"
		)
	};

	if walk.is_empty() {
		return Err("No images were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		walk.progress_crunch("Flaca", encode_image);
	}
	// Without progress.
	else {
		walk.process(encode_image);
	}

	Ok(())
}
