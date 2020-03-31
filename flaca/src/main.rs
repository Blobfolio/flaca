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
	Msg,
	Progress,
	progress_arc,
	witcher,
	PROGRESS_NO_ELAPSED
};
use fyi_core::witcher::mass::FYIMassOps;
use fyi_core::witcher::ops::FYIOps;
use rayon::prelude::*;
use std::path::PathBuf;
use std::time::Instant;



fn main() -> Result<(), String> {
	// Command line arguments.
	let opts: ArgMatches = menu::menu()
		.get_matches();

	// Check dependencies before doing anything else.
	flaca_core::check_dependencies();

	let pattern = witcher::pattern_to_regex(r"(?i).+\.(jpe?g|png)$");

	// What path are we dealing with?
	let paths: Vec<PathBuf> = match opts.is_present("list") {
		false => opts.values_of("path").unwrap()
			.into_iter()
			.filter_map(|x| Some(PathBuf::from(x)))
			.collect::<Vec<PathBuf>>()
			.fyi_walk_filtered(&pattern),
		true => PathBuf::from(opts.value_of("list").unwrap_or(""))
			.fyi_walk_file_lines(Some(pattern)),
	};

	if paths.is_empty() {
		return Err("No images were found.".to_string());
	}

	// With progress.
	if opts.is_present("progress") {
		let time = Instant::now();
		let before: u64 = paths.fyi_file_sizes();
		let found: u64 = paths.len() as u64;

		{
			use std::thread;
			let bar = Progress::new("", found, PROGRESS_NO_ELAPSED);
			let bar1 = bar.clone();

			let handler = thread::spawn(|| progress_arc::looper(bar1));
			paths.par_iter().for_each(|ref x| {
				let _ = x.flaca_encode().is_ok();

				progress_arc::set_path(bar.clone(), &x);
				progress_arc::increment(bar.clone(), 1);
			});
			progress_arc::finish(bar.clone());
			handler.join().unwrap();
		}

		let after: u64 = paths.fyi_file_sizes();
		Msg::msg_crunched_in(found, time, Some((before, after)))
			.print();
	}
	// Without progress.
	else {
		paths.par_iter().for_each(|ref x| {
			let _ = x.flaca_encode().is_ok();
		});
	}

	Ok(())
}
