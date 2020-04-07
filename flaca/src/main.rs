/*!
# Flaca

Brute-force, lossless JPEG and PNG compression.
*/

extern crate clap;
extern crate flaca_core;
extern crate fyi_core;
extern crate rayon;
extern crate regex;

mod menu;

use clap::ArgMatches;
use flaca_core::image::ImagePath;
use fyi_core::{
	arc::progress as parc,
	Msg,
	Prefix,
	Progress,
	PROGRESS_CLEAR_ON_FINISH,
	Witch,
};
use rayon::prelude::*;
use std::{
	path::PathBuf,
	time::Instant,
};



fn main() -> Result<(), String> {
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
				.filter_map(|x| Some(PathBuf::from(x)))
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
		return Err("No images were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		let time = Instant::now();
		let before: u64 = walk.du();
		let found: u64 = walk.len() as u64;

		{
			let bar = Progress::new(
				Msg::new("Reticulating splines…")
					.with_prefix(Prefix::Custom("Flaca", 199))
					.to_string(),
				found,
				PROGRESS_CLEAR_ON_FINISH
			);
			let looper = parc::looper(&bar, 60);
			walk.files().as_ref().par_iter().for_each(|x| {
				parc::add_working(&bar, &x);
				let _ = x.flaca_encode().is_ok();
				parc::update(&bar, 1, None, Some(x.to_path_buf()));
			});
			parc::finish(&bar);
			looper.join().unwrap();
		}

		let after: u64 = walk.du();
		Msg::msg_crunched_in(found, time, Some((before, after)))
			.print();
	}
	// Without progress.
	else {
		walk.files().as_ref().par_iter().for_each(|ref x| {
			let _ = x.flaca_encode().is_ok();
		});
	}

	Ok(())
}
