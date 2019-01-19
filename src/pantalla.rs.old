// Flaca: Pantalla
//
// Output and display helpers.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use ansi_term::{Colour, Style};
use crate::granjero::Cosecha;
use crate::lugar::Lugar;
use crate::mundo::Mundo;
use crate::VERSION;
use std::fmt;
use std::io::{Error, ErrorKind, Write};
use std::sync::Mutex;
use std::time::SystemTime;

#[derive(Clone, Copy, Debug, PartialEq)]
/// Display mode verbosity.
pub enum ModeKind {
	Debug,
	List,
	Standard,
	Quiet,
}

#[derive(Clone, Copy, Debug, PartialEq)]
/// Notice type.
pub enum LevelKind {
	Error,
	Info,
	Notice,
	Success,
	Warning,
}

impl fmt::Display for LevelKind {
	/// Display format.
	///
	/// This uses the canonical path when possible, but falls back to
	/// whatever was used to seed the PathBuf.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(
			f,
			"{}",
			match *self {
				LevelKind::Error => Colour::Red.bold().paint("Error:"),
				LevelKind::Info => Colour::Cyan.bold().paint("Info:"),
				LevelKind::Notice => Colour::Purple.bold().paint("Notice:"),
				LevelKind::Success => Colour::Green.bold().paint("Success:"),
				LevelKind::Warning => Colour::Yellow.bold().paint("Warning:"),
			}
		)
	}
}

#[derive(Debug, PartialEq, Clone)]
/// Display helper.
pub struct Pantalla {
	mode: ModeKind,
	time: SystemTime,
	log: Option<Lugar>,
	msg: String,
	tick: u64,
	total: u64,
}

impl Pantalla {
	// -----------------------------------------------------------------
	// Output Helpers
	// -----------------------------------------------------------------

	/// Print an error and exit.
	pub fn error(&self, text: Error) {
		self.print(format!(
			"{} {}",
			LevelKind::Error,
			text,
		), LevelKind::Error);
		std::process::exit(1);
	}

	/// Print a warning.
	pub fn warning(&self, text: Error) {
		self.print(format!(
			"{} {}",
			LevelKind::Warning,
			text,
		), LevelKind::Warning);
	}

	/// Print a debug notice.
	pub fn info(&self, text: String) {
		self.print(format!(
			"{} {}",
			LevelKind::Info,
			text,
		), LevelKind::Info);
	}

	/// Print a notice.
	pub fn notice(&self, text: String) {
		self.print(format!(
			"{} {}",
			LevelKind::Notice,
			text,
		), LevelKind::Notice);
	}

	/// Print a success.
	pub fn success(&self, text: String) {
		self.print(format!(
			"{} {}",
			LevelKind::Success,
			text,
		), LevelKind::Success);
	}

	/// Print a line.
	///
	/// This considers whether or not a progress bar exists, allowing
	/// records to be inserted before it.
	pub fn print(&self, text: String, level: LevelKind) {
		if self.can_print(level) {
			// Print the message bit.
			match level {
				LevelKind::Error => eprintln!("{}", text),
				LevelKind::Warning => eprintln!("{}", text),
				_ => println!("{}", Pantalla::pad_row(text)),
			}

			// Print the progress bar again if needed.
			self.print_progress(true);
		}
	}

	/// Print the progress bar.
	pub fn print_progress(&self, force: bool) {
		lazy_static! {
			static ref LAST: Mutex<String> = Mutex::new("".to_string());
		}

		let line: String = self.bar();
		let mut last = LAST.lock().unwrap();
		if ! self.has_progress() || (false == force && line == *last) {
			return;
		}

		// Spit it out.
		*last = line.to_string();
		eprint!("{}\r", Pantalla::pad_row(line));
	}

	// -----------------------------------------------------------------
	// State
	// -----------------------------------------------------------------

	/// Should output of a given notice level be displayed?
	fn can_print(&self, level: LevelKind) -> bool {
		match self.mode {
			ModeKind::Debug => true,
			ModeKind::List => false,
			ModeKind::Standard => match level {
				LevelKind::Info => false,
				_ => true,
			},
			ModeKind::Quiet => false,
		}
	}

	/// Whether or not there is a progress bar.
	fn has_progress(&self) -> bool {
		self.total > 0 && (self.mode == ModeKind::Debug || self.mode == ModeKind::Standard)
	}

	// -----------------------------------------------------------------
	// Data
	// -----------------------------------------------------------------

	/// The collective progress bar.
	fn bar(&self) -> String {
		if ! self.has_progress() {
			return "".to_string();
		}

		format!(
			"{} {} {} {} {}",
			self.bar_elapsed(),
			self.bar_bar(),
			self.bar_progress(),
			self.bar_percent(),
			self.bar_msg(),
		)
	}

	/// The bar part of the progress bar.
	fn bar_bar(&self) -> String {
		// Figure out the bar widths.
		let width: u64 = 50;
		let width1: u64 = match self.total {
			0 => 0,
			_ => (self.tick as f64 / self.total as f64 * width as f64).floor() as u64,
		};
		let width2: u64 = width - width1;

		// Draw up the bar strings.
		let bar1: String = match width1 {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x as usize]).unwrap_or("".to_string()),
		};
		let bar2: String = match width2 {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x as usize]).unwrap_or("".to_string()),
		};

		// Return a right-looking bar!
		format!(
			"{}{}",
			Colour::Cyan.bold().paint(bar1),
			Colour::Blue.paint(bar2),
		)
	}

	/// The time elapsed part of the progress bar.
	fn bar_elapsed(&self) -> String {
		if let Ok(x) = Lugar::time_diff(SystemTime::now(), self.time) {
			return Pantalla::nice_time(x, true);
		}

		"00:00:00".to_string()
	}

	/// The message part of the progress bar.
	fn bar_msg(&self) -> String {
		return self.msg.trim().to_string();
	}

	/// The percent part of the progress bar.
	fn bar_percent(&self) -> String {
		if 0 == self.total {
			return "  0%".to_string();
		}

		format!("{:>3.*}%", 0, self.tick as f64 / self.total as f64 * 100.0)
	}

	/// The x/y part of the progress bar.
	fn bar_progress(&self) -> String {
		let len: u64 = format!("{}", self.total).len() as u64;
		let done: String = Pantalla::pad_left(format!("{}", self.tick), len, b' ');

		format!(
			"{}/{}",
			Colour::Cyan.bold().paint(done),
			Colour::Blue.paint(format!("{}", self.total)),
		)
	}

	/// Get the combined length of the progress and percentage.
	///
	/// This is a one-off used to help us align starting bytes with the
	/// progress bar line.
	pub fn bar_progress_len(&self) -> u64 {
		format!("{}", self.total).len() as u64 * 2 + 6
	}

	/// Return the display mode.
	pub fn mode(&self) -> ModeKind {
		self.mode
	}

	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Set the display mode.
	pub fn set_mode(&mut self, mode: ModeKind) {
		self.mode = mode;
	}

	/// Set the log path.
	pub fn set_log(&mut self, log: Option<Lugar>) {
		if let Some(mut x) = log {
			if x.is_dir() {
				if let Ok(_) = x.push("flaca.log") {
					self.log = Some(x);
				}
			}
		}
	}

	/// Set the progress bar message.
	pub fn set_msg(&mut self, msg: String) {
		if msg != self.msg {
			self.msg = msg;
			self.print_progress(false);
		}
	}

	/// Set the progress bar tick.
	pub fn set_tick(&mut self, mut tick: u64) {
		if tick > self.total {
			tick = self.total;
		}

		if tick != self.tick {
			self.tick = tick;
			self.print_progress(false);
		}
	}

	/// Set the progress bar total.
	pub fn set_total(&mut self, total: u64) {
		if total != self.total {
			self.total = total;

			if self.total < self.tick {
				self.set_tick(self.total);
			}
			else {
				self.print_progress(false);
			}
		}
	}

	/// Log a result.
	pub fn log_result(&self, result: &mut Cosecha) -> Result<(), Error> {
		// Only run this if there is a log set.
		if self.log.is_none() {
			return Ok(());
		}

		let log = self.to_owned().log.unwrap();
		let name = log.name()?;
		if "flaca.log".to_string() != name {
			return Err(Error::new(ErrorKind::InvalidInput, "Invalid log path."));
		}

		let image = result.path();
		let path = image.canonical()?;
		let end_size: u64 = image.size()?;
		let start_size: u64 = end_size + result.saved();
		let status: String =
			if start_size > end_size {
				format!(
					"Saved {} bytes in {} seconds.",
					result.saved(),
					result.elapsed(),
				)
			}
			else { "No change.".to_string() };

		// Open/create the log file.
		let mut file = std::fs::OpenOptions::new()
			.write(true)
			.append(true)
			.create(true)
			.open(log.as_path_buf())?;

		// Append the line.
		if let Err(_) = writeln!(
			file,
			"{} {} -- {} {} -- {}",
			Lugar::local_now().to_rfc3339(),
			path,
			start_size,
			end_size,
			status,
		) {
			return Err(Error::new(ErrorKind::Other, "Unable to log results.").into());
		}

		Ok(())
	}

	// -----------------------------------------------------------------
	// Misc Helpers
	// -----------------------------------------------------------------

	/// Return a singular or plural string given the count.
	pub fn inflect(count: u64, singular: String, plural: String) -> String {
		match count {
			1 => singular,
			_ => plural,
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
		let mut s: f64 = time as f64;
		let d: f64 = (s / 86400.0).floor();
		s -= d * 86400.0;
		let h: f64 = (s / 3600.0).floor();
		s -= h * 3600.0;
		let m: f64 = (s / 60.0).floor();
		s -= m * 60.0;

		// Combine the strings.
		let mut out: Vec<String> = Vec::new();

		// Return a shortened version.
		if true == short {
			if d > 0.0 {
				out.push(format!("{:02}", d as u64));
			}

			// Always do hours, minutes, and seconds.
			out.push(format!("{:02}", h as u64));
			out.push(format!("{:02}", m as u64));
			out.push(format!("{:02}", s as u64));

			return out.join(":");
		}

		// A longer version.
		for (count, singular, plural) in vec![
			(d as u64, "day", "days"),
			(h as u64, "hour", "hours"),
			(m as u64, "minute", "minutes"),
			(s as u64, "second", "seconds"),
		] {
			if count > 0 {
				out.push(format!(
					"{} {}",
					count,
					Pantalla::inflect(count, singular.to_string(), plural.to_string()),
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

	/// Determine the length of padding needed.
	fn pad_len(text: &str, mut len: u64) -> u64 {
		if 0 == len {
			// Use the terminal width if it is higher.
			if let Some((w, _)) = term_size::dimensions() {
				if w as u64 > len {
					len = w as u64 - 1;
				}
			}
		}

		if text.len() as u64 >= len {
			return 0;
		}

		len - text.len() as u64
	}

	/// Pad a string on the left.
	pub fn pad_left(text: String, pad_length: u64, pad_string: u8) -> String {
		match Pantalla::pad_len(&text, pad_length) {
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
		match Pantalla::pad_len(&text, pad_length) {
			0 => text,
			x => format!(
				"{}{}",
				text,
				String::from_utf8(vec![pad_string; x as usize])
					.unwrap_or("".to_string()),
			),
		}
	}

	/// Pad a line so it stretches the width of the screen.
	fn pad_row(text: String) -> String {
		Pantalla::pad_right(text, 0, b' ')
	}

	pub fn chop_left(text: String, limit: u64) -> String {
		let text_len: u64 = text.len() as u64;
		if text_len <= limit {
			return text;
		}

		let diff: u64 = text_len - limit + 1;
		let short: String = text.chars().skip(diff as usize).collect();
		format!("…{}", short)
	}

	pub fn chop_right(text: String, limit: u64) -> String {
		let text_len: u64 = text.len() as u64;
		if text_len <= limit {
			return text;
		}

		let diff: u64 = text_len - limit + 1;
		let short: String = text.chars().take(diff as usize).collect();
		format!("{}…", short)
	}

	// -----------------------------------------------------------------
	// Screens
	// -----------------------------------------------------------------

	/// A fun little CLI introductory header.
	pub fn header(&self, count: u64, size: u64) {
		// Don't print if we're supposed to be quiet.
		if ! self.can_print(LevelKind::Success) {
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
			Colour::Purple.bold().paint("Flaca"),
			Style::new().bold().paint(format!("v{}", VERSION)),
			Colour::Blue.bold().paint("Images:"),
			count,
			Colour::Blue.bold().paint("Space:"),
			Pantalla::nice_size(size),
			"Ready, Set, Goat!",
		);
	}

	/// Print the runtime settings being used.
	pub fn settings(&self, opts: &Mundo) {
		if self.mode != ModeKind::Debug {
			return;
		}

		self.info("Debug enabled; expect verbose output!".to_string());
		self.info(format!("Flaca started at {}", Lugar::local_now().to_rfc3339()));

		if let Some(ref x) = self.log {
			self.info(format!("Logging results to {}.", x));
		}

		if let Some(x) = opts.skip() {
			self.info(format!("Skipping {} images.", x));
		}

		if let Some(x) = opts.min_age() {
			self.info(format!(
				"Skipping files younger than {}.",
				Pantalla::nice_time(x, false)
			));
		}

		if let Some(x) = opts.max_age() {
			self.info(format!(
				"Skipping files Older than {}.",
				Pantalla::nice_time(x, false)
			));
		}

		if let Some(x) = opts.min_size() {
			self.info(format!(
				"Skipping files smaller than {}.",
				Pantalla::nice_size(x)
			));
		}

		if let Some(x) = opts.max_size() {
			self.info(format!(
				"Skipping files larger than {}.",
				Pantalla::nice_size(x)
			));
		}

		for encoder in opts.jpg().iter() {
			if let Ok(x) = encoder.cmd() {
				self.info(format!(
					"Found {} at {}",
					encoder.name(),
					x
				));
			}
		}

		for encoder in opts.png().iter() {
			if let Ok(x) = encoder.cmd() {
				self.info(format!(
					"Found {} at {}",
					encoder.name(),
					x
				));
			}
		}

		println!("");
	}

	/// List found images and exit.
	pub fn list(&self, images: &Vec<Lugar>) {
		if self.mode != ModeKind::List {
			return;
		}

		for ref i in images {
			if let Ok(x) = i.canonical() {
				println!("{}", x);
			}
		}

		std::process::exit(0);
	}

	/// Summarize total results.
	pub fn footer(&self, count: u64, saved: u64) {
		// Don't print if we're supposed to be quiet.
		if ! self.can_print(LevelKind::Success) {
			return;
		}

		// Print a blank line.
		println!("");
		println!("");

		// Total images.
		println!(
			"{} Chewed {} {}.",
			LevelKind::Info,
			count,
			Pantalla::inflect(count, "image".to_string(), "images".to_string()),
		);

		// Total runtime.
		println!(
			"{} Flaca ran for {}.",
			LevelKind::Info,
			Pantalla::nice_time(
				Lugar::time_diff(SystemTime::now(), self.time).unwrap_or(0),
				false,
			),
		);

		// Report savings, if any.
		if saved > 0 {
			println!(
				"{} Saved {}!",
				LevelKind::Success,
				Pantalla::nice_size(saved),
			);
		}
		else {
			println!(
				"{} No space was freed. {}",
				LevelKind::Warning,
				Colour::Red.bold().paint(":("),
			);
		}
	}
}

impl Default for Pantalla {
	fn default() -> Pantalla {
		Pantalla {
			mode: ModeKind::Standard,
			time: SystemTime::now(),
			log: None,
			msg: "".to_string(),
			tick: 0,
			total: 0,
		}
	}
}
