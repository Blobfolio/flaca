// Flaca: Diario
//
// Output, display, progress, logging, etc.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use crate::lugar::Lugar;
use std::fmt;
use std::io::Error;
use std::time::SystemTime;

// TODO: Log
// TODO: Progress Bar Bits

#[derive(Clone, Debug, PartialEq)]
pub enum Note {
	Error,
	Info,
	Notice,
	Success,
	Warning,
}

impl fmt::Display for Note {
	/// Display format.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Note::Error => write!(f, "{}", ansi_term::Colour::Red.bold().paint("Error:")),
			Note::Info => write!(f, "{}", ansi_term::Colour::Cyan.bold().paint("Info:")),
			Note::Notice => write!(f, "{}", ansi_term::Colour::Purple.bold().paint("Notice:")),
			Note::Success => write!(f, "{}", ansi_term::Colour::Green.bold().paint("Success:")),
			Note::Warning => write!(f, "{}", ansi_term::Colour::Yellow.bold().paint("Warning:")),
		}
	}
}

#[derive(Clone, Debug, PartialEq)]
pub enum Verbosity {
	Debug,
	Standard,
	Quiet,
}

#[derive(Clone, Debug)]
/// An ergonomic path wrapper.
pub struct Diario {
	mode: Verbosity,
	time: SystemTime,
	msg: String,
	tick: u64,
	total: u64,
}

impl Default for Diario {
	fn default() -> Diario {
		Diario {
			mode: Verbosity::Standard,
			time: SystemTime::now(),
			msg: "".to_string(),
			tick: 0,
			total: 0,
		}
	}
}

impl Diario {
	pub fn new(verbosity: Verbosity) -> Diario {
		Diario {
			mode: verbosity,
			..Diario::default()
		}
	}

	pub fn set_total(&mut self, total: u64) {
		self.total = total;
	}

	pub fn set_tick(&mut self, tick: u64) {
		self.tick = tick;
	}

	pub fn set_msg(&mut self, msg: String) {
		self.msg = msg;
	}

	pub fn error(&self, text: Error) {
		eprintln!(
			"{} {}",
			Note::Error,
			text,
		);
		std::process::exit(1);
	}

	pub fn info(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Note::Info,
			text,
		));
	}

	pub fn notice(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Note::Notice,
			text,
		));
	}

	pub fn success(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Note::Success,
			text,
		));
	}

	pub fn warning(&self, text: Error) {
		self.progressln(format!(
			"{} {}",
			Note::Warning,
			text,
		));
	}

	pub fn progressln(&self, text: String) {
		println!("{}", text);

		// Redraw progress bar.
		if self.mode != Verbosity::Quiet {

		}
	}

	pub fn inflect(count: u64, singular: String, plural: String) -> String {
		match count {
			1 => singular,
			_ => plural,
		}
	}

	fn pad_len(text: &str, mut len: u64) -> u64 {
		// Use the terminal width if it is higher.
		if let Some((w, _)) = term_size::dimensions() {
			if w as u64 > len {
				len = w as u64;
			}
		}

		match text.len() as u64 >= len {
			true => 0,
			false => len - text.len() as u64,
		}
	}

	pub fn pad_left(text: String, pad_length: u64, pad_string: u8) -> String {
		match Diario::pad_len(&text, pad_length) {
			0 => text,
			x => format!(
				"{}{}",
				String::from_utf8(vec![pad_string; x as usize])
					.unwrap_or("".to_string()),
				text,
			),
		}
	}

	pub fn pad_right(text: String, pad_length: u64, pad_string: u8) -> String {
		match Diario::pad_len(&text, pad_length) {
			0 => text,
			x => format!(
				"{}{}",
				text,
				String::from_utf8(vec![pad_string; x as usize])
					.unwrap_or("".to_string()),
			),
		}
	}

	/// Convert a byte size into a more human-friendly unit, like 2.4MB.
	pub fn nice_size(size: u64) -> String {
		if size <= 0 {
			return "0B".to_string();
		}

		// Gigabytes.
		if size >= 943718400 {
			return format!("{:.*}GB", 2, size as f64 / 1073741824 as f64);
		}
		// Megabytes.
		else if size >= 921600 {
			return format!("{:.*}MB", 2, size as f64 / 1048576 as f64);
		}
		// Kilobytes.
		else if size >= 900 {
			return format!("{:.*}KB", 2, size as f64 / 1024 as f64);
		}

		format!("{}B", size)
	}

	/// Format seconds either as a 00:00:00 counter or broken out into a
	/// list of hours, minutes, etc.
	pub fn nice_time(time: u64, short: bool) -> String {
		if time <= 0 {
			match short {
				true => return "00:00:00".to_string(),
				false => return "0 seconds".to_string(),
			}
		}

		// Drill down to days, hours, minutes, and seconds.
		let mut s: u64 = time;
		let d: u64 = (s as f64 / 86400 as f64).floor() as u64;
		s -= d * 86400;
		let h: u64 = (s as f64 / 3600 as f64).floor() as u64;
		s -= h * 3600;
		let m: u64 = (s as f64 / 60 as f64).floor() as u64;
		s -= m * 60;

		// Combine the strings.
		let mut out: Vec<String> = Vec::new();

		// Return a shortened version.
		if true == short {
			if d > 0 {
				out.push(format!("{:02}", d));
			}

			// Always do hours, minutes, and seconds.
			out.push(format!("{:02}", h));
			out.push(format!("{:02}", m));
			out.push(format!("{:02}", s));

			return out.join(":");
		}

		// A longer version.
		for (count, singular, plural) in vec![
			(d, "day", "days"),
			(h, "hour", "hours"),
			(m, "minute", "minutes"),
			(s, "second", "seconds"),
		] {
			if count > 0 {
				out.push(format!(
					"{} {}",
					count,
					Diario::inflect(count, singular.to_string(), plural.to_string()),
				));
			}
		}

		match out.len() {
			1 => out.pop().unwrap_or("0 seconds".to_string()),
			2 => out.join(" and "),
			_ => {
				let last = out.pop().unwrap_or("".to_string());
				format!("{}, and {}", out.join(", "), last)
			},
		}
	}
}
