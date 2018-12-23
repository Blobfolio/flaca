// Flaca: Diario
//
// Output, display, progress, logging, etc.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use chrono::TimeZone;
use crate::granjero::Cosecha;
use crate::lugar::Lugar;
use crate::mundo::Mundo;
use crate::VERSION;
use std::fmt;
use std::io::{Error, ErrorKind, Write};
use std::path::PathBuf;
use std::time::SystemTime;

// TODO: Progress Bar Bits

#[derive(Clone, Copy, Debug, PartialEq)]
/// Common notice types.
pub enum Noticia {
	Error,
	Info,
	Notice,
	Success,
	Warning,
}

impl fmt::Display for Noticia {
	/// Display format.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Noticia::Error => write!(f, "{}", ansi_term::Colour::Red.bold().paint("Error:")),
			Noticia::Info => write!(f, "{}", ansi_term::Colour::Cyan.bold().paint("Info:")),
			Noticia::Notice => write!(f, "{}", ansi_term::Colour::Purple.bold().paint("Notice:")),
			Noticia::Success => write!(f, "{}", ansi_term::Colour::Green.bold().paint("Success:")),
			Noticia::Warning => write!(f, "{}", ansi_term::Colour::Yellow.bold().paint("Warning:")),
		}
	}
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Reporting verbosity level.
pub enum Nivel {
	Debug,
	List,
	Standard,
	Quiet,
}

#[derive(Clone, Debug, PartialEq)]
/// An ergonomic path wrapper.
pub struct Diario {
	pub mode: Nivel,
	pub time: SystemTime,
	pub log: Option<Lugar>,
	pub msg: String,
	pub tick: u64,
	pub total: u64,
}

impl Default for Diario {
	fn default() -> Diario {
		Diario {
			mode: Nivel::Standard,
			time: SystemTime::now(),
			log: None,
			msg: "".to_string(),
			tick: 0,
			total: 0,
		}
	}
}

impl Diario {
	/// New output driver.
	pub fn new(verbosity: Nivel, log: Option<Lugar>) -> Diario {
		Diario {
			mode: verbosity,
			log: match log {
				Some(x) => {
					if ! x.is_dir() {
						None
					} else {
						Some(x)
					}
				},
				None => None,
			},
			..Diario::default()
		}
	}

	/// Set the progress bar total.
	pub fn set_total(&mut self, total: u64) {
		self.total = total;
	}

	/// Set the progress bar current tick.
	pub fn set_tick(&mut self, tick: u64) {
		self.tick = tick;
	}

	/// Set the progress bar message.
	pub fn set_msg(&mut self, msg: String) {
		self.msg = msg;
	}

	/// Print an error and exit.
	pub fn error(&self, text: Error) {
		eprintln!(
			"{} {}",
			Noticia::Error,
			text,
		);
		std::process::exit(1);
	}

	/// Print an informational tidbit.
	pub fn info(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Noticia::Info,
			text,
		));
	}

	/// Print a notice.
	pub fn notice(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Noticia::Notice,
			text,
		));
	}

	/// Print a success.
	pub fn success(&self, text: String) {
		self.progressln(format!(
			"{} {}",
			Noticia::Success,
			text,
		));
	}

	/// Print a warning.
	pub fn warning(&self, text: Error) {
		self.progressln(format!(
			"{} {}",
			Noticia::Warning,
			text,
		));
	}

	/// Print a line to the screen, possibly *before* the progress bar.
	pub fn progressln(&self, text: String) {
		println!("{}", text);

		// TODO Redraw progress bar.
		if
			self.mode != Nivel::Quiet &&
			self.mode != Nivel::List &&
			self.total > 0 &&
			self.total != self.tick {

		}
	}

	/// Return a singular or plural string given the count.
	pub fn inflect(count: u64, singular: String, plural: String) -> String {
		match count {
			1 => singular,
			_ => plural,
		}
	}

	/// Determine the length of padding needed.
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

	/// Pad a string on the left.
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

	/// Pad a string on the right.
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

	/// Get a datetime object in the local timezone.
	pub fn local_now() -> chrono::DateTime<chrono::Local> {
		let start = SystemTime::now();
		let start_since = start.duration_since(std::time::UNIX_EPOCH).expect("Time is meaningless.");

		chrono::Local.timestamp(start_since.as_secs() as i64, 0)
	}

	/// Log compression results for an image.
	pub fn log(&mut self, result: &mut Cosecha) -> Result<(), Error> {
		// Only applies if a log destination was specified.
		if ! self.log.is_some() {
			return Err(Error::new(ErrorKind::NotFound, "Missing log path."));
		}

		let root = self.to_owned().log.unwrap().canonical()?;
		let log = PathBuf::from(format!("{}/{}", root, "flaca.log"));

		let path = result.path.canonical()?;
		let saved = result.saved();
		let elapsed = result.elapsed();

		// Generate a human-friendly status based on what took place.
		let status: String = match saved {
			0 => "No change.".to_string(),
			_ => format!(
				"Saved {} bytes in {} seconds.",
				saved,
				elapsed,
			),
		};

		// Open/create the log file.
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			.append(true)
			.create(true)
			.open(log)?;

		// Append the line.
		if let Err(_) = writeln!(
			file,
			"{} \"{}\" {} {} {}",
			Diario::local_now().to_rfc3339(),
			path,
			result.start_size,
			result.end_size,
			status,
		) {
			return Err(Error::new(ErrorKind::Other, "Unable to log results.").into());
		}

		Ok(())
	}

	/// A fun little CLI introductory header.
	pub fn header(&self, count: u64, size: u64) {
		// Don't print if we're supposed to be quiet.
		if self.mode != Nivel::Quiet && self.mode != Nivel::List {
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
			count,
			ansi_term::Colour::Blue.bold().paint("Space:"),
			Diario::nice_size(size),
			"Ready, Set, Goat!",
		);
	}

	/// Debug runtime settings.
	pub fn debug(&self, opts: &mut Mundo) {
		// Don't print if we're supposed to be quiet.
		if self.mode != Nivel::Debug {
			return;
		}

		self.info("Debug enabled; expect verbose output!".to_string());
		self.info(format!("Flaca started at {}", Diario::local_now().to_rfc3339()));

		match self.to_owned().log {
			Some(x) => {
				self.info(format!(
					"Logging to '{}/flaca.log'.",
					x.canonical().unwrap_or("MISSING".to_string())
				));
			},
			_ => {},
		};

		if let Some(x) = opts.skip {
			self.info(format!("Skipping {} files.", x));
		}

		if let Some(x) = opts.min_age {
			self.info(format!(
				"Skipping files younger than {}.",
				Diario::nice_time(x, false)
			));
		}

		if let Some(x) = opts.max_age {
			self.info(format!(
				"Skipping files Older than {}.",
				Diario::nice_time(x, false)
			));
		}

		if let Some(x) = opts.min_size {
			self.info(format!(
				"Skipping files smaller than {}.",
				Diario::nice_size(x)
			));
		}

		if let Some(x) = opts.max_size {
			self.info(format!(
				"Skipping files larger than {}.",
				Diario::nice_size(x)
			));
		}

		for encoder in opts.jpg.iter() {
			if let Ok(x) = encoder.cmd_path() {
				self.info(format!(
					"Found {} at {}",
					encoder.name(),
					x
				));
			}
		}

		for encoder in opts.png.iter() {
			if let Ok(x) = encoder.cmd_path() {
				self.info(format!(
					"Found {} at {}",
					encoder.name(),
					x
				));
			}
		}
	}

	/// List found images and exit.
	pub fn list(&self, opts: &mut Mundo) {
		if self.mode != Nivel::List {
			return;
		}

		for ref i in &opts.input {
			if let Ok(x) = i.canonical() {
				println!("{}", x);
			}
		}

		std::process::exit(0);
	}

	/// Summarize total results.
	pub fn summarize(&self, count: u64, saved: u64) {
		// Don't print if we're supposed to be quiet.
		if self.mode != Nivel::Quiet && self.mode != Nivel::List {
			return;
		}

		let end = SystemTime::now();
		let elapsed: u64 = match end.duration_since(self.time) {
			Ok(x) => x.as_secs(),
			_ => 0,
		};

		// Print a blank line.
		println!("");

		// Total images.
		println!(
			"{} Chewed {} {}.",
			Noticia::Info,
			count,
			Diario::inflect(count, "image".to_string(), "images".to_string()),
		);

		// Total runtime.
		println!(
			"{} Flaca ran for {}.",
			Noticia::Info,
			Diario::nice_time(elapsed, false),
		);

		// Report savings, if any.
		if saved > 0 {
			println!(
				"{} Saved {}!",
				Noticia::Success,
				Diario::nice_size(saved),
			);
		}
		else {
			println!(
				"{} No space was freed. {}",
				Noticia::Warning,
				ansi_term::Colour::Red.bold().paint(":("),
			);
		}
	}
}
