// Flaca
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>



#![warn(trivial_casts, trivial_numeric_casts, unused_import_braces)]
#![deny(missing_debug_implementations, missing_copy_implementations)]



extern crate ansi_term;
extern crate chrono;
extern crate clap;
#[macro_use]
extern crate lazy_static;
extern crate nix;
extern crate term_size;

pub mod granjero;
pub mod lugar;
pub mod mundo;
pub mod pantalla;

use ansi_term::Colour;
use crate::granjero::Cosecha;
use crate::lugar::Lugar;
use crate::mundo::Mundo;
use crate::pantalla::Pantalla;
use crate::pantalla::LevelKind;
use std::io::{Error, ErrorKind};
use std::time::SystemTime;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");



fn main() {
	let opts = Mundo::new();
	let mut display = opts.output();
	let images = opts.images();
	let images_len: u64 = opts.total_images();

	display.header(images_len, opts.total_size());
	display.settings(&opts);
	display.list(&images);

	// If there are no images, we're done.
	if 0 == images_len {
		display.error(Error::new(ErrorKind::NotFound, "No qualifying images were found."));
	}

	let related: String = format!("{}", Colour::Purple.bold().paint("    ↳"));
	let mut tick: u64 = 0;
	let mut saved: u64 = 0;
	let jpg_encoders = opts.jpg();
	let has_jpg: bool = jpg_encoders.len() != 0;
	let png_encoders = opts.png();
	let has_png: bool = png_encoders.len() != 0;
	display.set_total(images_len);

	// To help align displays, we want to know the size of some bar
	// bits.
	let progress_len: u64 = display.bar_progress_len();

	// Loop and compress!
	for ref i in &images {
		if ! i.is_image() {
			tick += 1;
			display.set_tick(tick);
			continue;
		}

		// Start a result file.
		let mut result = Cosecha::start(Lugar::new(i.canonical().unwrap()));

		// Print the image name and time started.
		display.print(format!(
			"{}",
			Colour::Purple.bold().paint(format!("[{}]", Lugar::local_now().to_rfc3339())),
		), LevelKind::Notice);

		display.print(format!(
			"{} {:53} {}",
			related,
			Pantalla::chop_left(format!("{}", i), 53),
			Colour::Blue.paint(Pantalla::pad_left(
				format!("{}", result.start_size()),
				progress_len,
				b' ',
			)),
		), LevelKind::Notice);

		// Make sure we can handle the image type.
		if (i.is_jpg() && false == has_jpg) || (i.is_png() && false == has_png) {

			if i.is_jpg() {
				display.warning(Error::new(ErrorKind::NotFound, "No JPEG encoders are available."));
			}
			else {
				display.warning(Error::new(ErrorKind::NotFound, "No PNG encoders are available."));
			}

			// Tick and reset.
			tick += 1;
			display.set_tick(tick);
			continue;
		}

		// Reference the encoder set so we can do a single loop for both.
		let encoders =
			if i.is_jpg() { &jpg_encoders }
			else { &png_encoders };

		// Loop the encoders.
		for ref e in encoders {
			let local_size: u64 = i.size().unwrap_or(0);
			let local_time = SystemTime::now();
			display.print_progress(false);

			// Print the image name and time started.
			display.print(format!(
				"{} Trying {} at {}.",
				related,
				e.name(),
				e.cmd().unwrap_or("MISSING".to_string())
			), LevelKind::Info);

			// Maybe the size changed.
			if let Ok(_) = e.compress(&mut result) {
				let now_size = i.size().unwrap_or(0);
				let now_saved =
					if local_size > 0 && now_size > 0 && local_size > now_size {
						local_size - now_size
					}
					else {
						0
					};
				let now_elapsed = Lugar::time_diff(SystemTime::now(), local_time).unwrap_or(0);

				if 0 == now_saved {
					display.print(format!(
						"{} {} => No change.",
						related,
						Pantalla::nice_time(now_elapsed, true),
					), LevelKind::Info);
				}
				else {
					saved += now_saved;
					display.set_msg(Colour::Green.bold().paint(
						format!("-{}", Pantalla::nice_size(saved))
					).to_string());

					display.print(format!(
						"{} {} => Saved {}.",
						related,
						Pantalla::nice_time(now_elapsed, true),
						Colour::Green.bold().paint(Pantalla::nice_size(now_saved)),
					), LevelKind::Info);
				}

				match now_size - i.size().unwrap_or(0) {
					0 => {},
					x => display.print(format!(
							"{} Saved {}",
							related,
							Colour::Green.bold().paint(Pantalla::nice_size(x)),
						), LevelKind::Info),
				}
			}
		}

		// Log result?
		if let Err(_) = display.log_result(&mut result) {}

		tick += 1;
		display.set_tick(tick);
	}

	display.footer(images_len, saved);
}
