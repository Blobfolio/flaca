/*!
# Display
*/

use ansi_term::{Colour, Style};
use crossbeam_channel::{Receiver, unbounded};
use flaca_core::{Config, FlacaError, Format, LogEntry, LogEntryKind, Reporter};
use Format::FormatKind;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};




#[derive(Debug, Clone)]
/// A Display.
pub struct Display {
	/// The reporter.
	reporter: Arc<Mutex<Reporter>>,
	/// The starting time.
	time: Instant,
	/// Last Log.
	last: Option<LogEntry>,
	/// Receiver.
	receiver: Receiver<LogEntry>,
	/// Last Tick.
	tock: String,
}

impl Display {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// New.
	pub fn new(reporter: Arc<Mutex<Reporter>>) -> Display {
		let (tx, rx) = unbounded();
		Reporter::arc_set_sender(reporter.clone(), Some(tx.clone()));

		Display {
			reporter: reporter.clone(),
			time: Instant::now(),
			last: None,
			receiver: rx,
			tock: "".to_string(),
		}
	}

	/// Reset.
	///
	/// Reset everything but the reporter.
	pub fn reset(&mut self) {
		self.time = Instant::now();
		self.last = None;
		self.tock = "".to_string();
	}



	// -----------------------------------------------------------------
	// Runtime
	// -----------------------------------------------------------------

	/// Print Error and Exit.
	pub fn die(&self, error: FlacaError) {
		// Print if we aren't being quiet.
		if 0 < Reporter::arc_level(self.reporter.clone()) {
			eprintln!(
				"{} {}",
				Colour::Red.bold().paint("Error:"),
				error
			);
		}

		// Exit!
		std::process::exit(1);
	}

	/// Start Display.
	pub fn watch(&mut self, config: Arc<Mutex<Config>>) {
		// If we're being quiet there's nothing to do.
		if 0 == Reporter::arc_level(self.reporter.clone()) {
			return;
		}

		// The reporter might not be ready yet.
		let now = Instant::now();
		let sleep = Duration::from_millis(50);
		loop {
			// It's ready.
			if 0 < Reporter::arc_total(self.reporter.clone()) {
				break;
			}
			// It has been five seconds; we should quit.
			else if 5 < now.elapsed().as_secs() {
				return;
			}

			// Sleep a while and check again.
			thread::sleep(sleep);
		}

		// Print the header.
		self.print_start(config.clone());

		// Now we can loop at a more leisurely pace while the reporter
		// is doing its thing.
		let sleep = Duration::from_millis(150);
		loop {
			// Tick.
			self.tick();

			// Rest.
			thread::sleep(sleep);

			// And break if the reporter has subsequently finished.
			if false == Reporter::arc_running(self.reporter.clone()) {
				break;
			}
		}

		// Do one more tick at the end to catch anything final messages
		// that might have come through.
		self.tick();

		// Erase the bar at the end to declare ourselves done!
		print!("{}", ansi_escapes::EraseLines(5));
		if let Some(last) = self.last.as_ref() {
			println!("{}", Display::format_log_entry(last.clone()));
		}
	}

	/// Tick.
	fn tick(&mut self) {
		let elapsed = Format::time::human_elapsed(self.time, FormatKind::Short);
		if elapsed != self.tock {
			self.tock = elapsed;
			if self.last.is_some() {
				self.print_bar();
			}
		}

		while let Ok(log) = self.receiver.try_recv() {
			self.set_log(log);
		}
	}

	/// Update the Log.
	fn set_log(&mut self, entry: LogEntry) {
		// Update and deal with the past.
		if let Some(old) = self.last.replace(entry.clone()) {
			print!("{}", ansi_escapes::EraseLines(5));

			// Re-print the old line with dimmed styles.
			println!("{}", Display::format_old_log_entry(old));

			// Give us a little separator bar.
			Display::print_divider();
		}

		// Print the entry.
		println!("{}", Display::format_log_entry(entry));
		Display::print_divider();
		println!("");

		// Send to the printer.
		self.print_bar();
	}



	// -----------------------------------------------------------------
	// Printing
	// -----------------------------------------------------------------

	/// Print Progress Bar.
	fn print_bar(&self) {
		let done: usize = Reporter::arc_done(self.reporter.clone());
		let total: usize = Reporter::arc_total(self.reporter.clone());

		print!("{}", ansi_escapes::EraseLines(2));
		println!("{}", Display::format_bar(done, total, self.tock.clone()));
	}

	/// Print Divider.
	fn print_divider() {
		println!(
			"{}",
			Style::new().dimmed().paint(
				Format::strings::pad_left("", Display::width() - 5, b'-')
			)
		);
	}

	/// Print Header.
	fn print_start(&self, config: Arc<Mutex<Config>>) {
		// Figure out the total.
		let total: usize = Reporter::arc_total(self.reporter.clone());
		let label: String = match total {
			0 => "",
			_ => "Images:",
		}.to_string();

		// And grab the image app list.
		let apps: String = Format::grammar::oxford_join(
			Config::arc_image_app_list(config.clone()),
			"and"
		);

		// We'll also want to note dry runs.
		let dry_run: String = match Reporter::arc_dry_run(self.reporter.clone()) {
			true => Colour::Yellow.bold().paint("(This is just a dry run.)").to_string(),
			false => "".to_string(),
		};

		println!("
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
      \\--','.        ;  {} {}
       `;;  :       /
        ;'  ;  ,' ,'    {}
        ,','  :  '      {}
        \\ \\   :
         `
",
			Colour::Purple.bold().paint("Flaca"),
			Style::new().bold().paint(format!("v{}", env!("CARGO_PKG_VERSION"))),
			Colour::Blue.bold().paint(label),
			total,
			Colour::Cyan.bold().paint("Using:"),
			apps,
			"Ready, Set, Goat!",
			dry_run
		);
	}



	// -----------------------------------------------------------------
	// Formatting
	// -----------------------------------------------------------------

	/// Strip Styles
	fn strip_styles<S> (text: S) -> String
	where S: Into<String> {
		let text = text.into();
		let stripped = strip_ansi_escapes::strip(text).unwrap_or(Vec::new());
		std::str::from_utf8(&stripped).unwrap_or("").to_string()
	}

	/// Format LogEntry.
	fn format_log_entry(entry: LogEntry) -> String {
		// This is pretty terrible. Haha. Let's start by gathering the
		// individual pieces and counting up their lengths.
		let prefix: String = Display::format_log_entry_prefix(entry.kind);
		let prefix_len: usize = Display::format_len(&prefix);

		let mut msg: String = Display::format_log_entry_msg(&entry);
		let mut msg_len: usize = Display::format_len(&msg);

		let mut path: String = match entry.path {
			Some(ref p) => Format::path::as_string(&p),
			None => "".to_string(),
		};
		let path_len: usize = Display::format_len(&path);

		let date: String = Display::format_log_entry_date(&entry);
		let date_len: usize = date.len();

		// Now let's do a lot of width-related calculations!
		let total_len: usize = Display::width() - 5;

		// If the output is too long not considering the path, we'll
		// trim the message and call it a day. Note the magic number is
		// two for minimum spacing before the date, and 20 for a cap on
		// how small a space a path can be usefully shoved into.
		if total_len <= prefix_len + msg_len + date_len + 22 {
			// Shorten the message?
			if total_len <= prefix_len + msg_len + date_len + 2 {
				msg = Format::strings::shorten_right(
					&msg,
					total_len - prefix_len - date_len - 2
				);
				msg_len = Display::format_len(&msg);
			}

			format!(
				"{}{}{}",
				&prefix,
				&msg,
				Style::new().dimmed().paint(Format::strings::pad_left(
					&date,
					total_len - prefix_len - msg_len,
					b' '
				))
			)
		}
		// Otherwise we can print with the path.
		else {
			// Shorten the path?
			if total_len <= prefix_len + msg_len + path_len + date_len + 4 {
				path = Format::strings::shorten_left(
					&path,
					total_len - prefix_len - msg_len - date_len - 4
				);
			}

			format!(
				"{}{}  {}  {}",
				&prefix,
				&msg,
				Colour::Cyan.bold().paint(
					Format::strings::pad_left(
						&path,
						total_len - prefix_len - msg_len - date_len - 4,
						b' '
					)
				),
				Style::new().dimmed().paint(&date)
			)
		}
	}

	/// Format LogEntry Prefix.
	fn format_log_entry_prefix(kind: LogEntryKind) -> String {
		match kind {
			// Debug and log notices are purple.
			LogEntryKind::Debug | LogEntryKind::Notice => format!(
				"{}",
				Colour::Purple.bold().paint(kind.prefix())
			),
			// Success is green.
			LogEntryKind::Success => format!(
				"{}",
				Colour::Green.bold().paint(kind.prefix())
			),
			// Warning is yellow.
			LogEntryKind::Warning => format!(
				"{}",
				Colour::Yellow.bold().paint(kind.prefix())
			),
			// Errors are red.
			LogEntryKind::Error => format!(
				"{}",
				Colour::Red.bold().paint(kind.prefix())
			),
			// Pass-through messages have no prefix.
			LogEntryKind::Other => "".to_string(),
		}
	}

	/// Format LogEntry Message.
	fn format_log_entry_msg(entry: &LogEntry) -> String {
		// Build the message part.
		let mut msg: String = entry.msg.to_string();
		if let Some(saved) = entry.saved {
			match saved {
				0 => msg = format!("{} No change.", msg),
				_ => msg = format!("{} Saved {}!", msg, Format::path::human_size(saved)),
			}
		}

		msg.trim().to_string()
	}

	/// Format LogEntry Date.
	fn format_log_entry_date(entry: &LogEntry) -> String {
		format!("{}", entry.date.format("%T")).trim().to_string()
	}

	/// Format LogEntry Date.
	fn format_old_log_entry_date(entry: &LogEntry) -> String {
		format!("{}", entry.date.format("%F %T")).trim().to_string()
	}

	/// Format Old LogEntry.
	///
	/// We have a different display style for older records.
	fn format_old_log_entry(entry: LogEntry) -> String {
		let prefix: String = Display::strip_styles(entry.kind.prefix());
		let prefix_len: usize = Display::format_len(&prefix);

		let mut msg: String = Display::strip_styles(Display::format_log_entry_msg(&entry));
		let msg_len: usize = Display::format_len(&msg);

		let date: String = Display::strip_styles(Display::format_old_log_entry_date(&entry));
		let date_len: usize = date.len();

		// Now let's do a lot of width-related calculations!
		let total_len: usize = Display::width() - 5;

		// We might need to chop the message.
		if total_len <= prefix_len + msg_len + date_len + 3 {
			msg = Format::strings::shorten_right(
				&msg,
				total_len - prefix_len - date_len - 3
			);
		}

		let line1 = format!(
			"[{}] {}{}",
			Colour::Purple.bold().paint(&date),
			Style::new().paint(&prefix),
			Style::new().dimmed().paint(&msg),
		);

		// The path, if any, will be moved to a new line.
		let mut path: String = match entry.path {
			Some(ref p) => Format::path::as_string(&p),
			None => return line1,
		};
		let path_len: usize = Display::format_len(&path);

		// Shorten the path?
		if total_len <= 6 + path_len {
			path = Format::strings::shorten_left(
				&path,
				total_len - 6
			);
		}

		// Glue them together!
		format!(
			"{}\n  ↳ {}",
			line1,
			Style::new().dimmed().paint(&path)
		)
	}

	/// Format Length.
	///
	/// This calculates the length of a string minus any ANSI escapes
	/// that might be taking up "space".
	fn format_len<S> (text: S) -> usize
	where S: Into<String> {
		let stripped: String = Display::strip_styles(text.into());
		stripped.chars().count()
	}

	/// Format Progress
	fn format_bar(done: usize, total: usize, elapsed: String) -> String {
		let elapsed_len: usize = elapsed.len();
		let progress: f64 = match total {
			0 => 0.0,
			_ => done as f64 / total as f64,
		};

		// The numbers: done/total.
		let progress_num: String = format!(
			"{}/{}",
			Colour::Cyan.bold().paint(format!("{}", done)),
			Colour::Cyan.dimmed().paint(format!("{}", total)),
		);
		let progress_num_len: usize = Display::format_len(&progress_num);

		// The numbers as a percentage.
		let progress_percent: String = format!(
			"{}",
			Colour::White.bold().paint(format!("{:>3.*}%", 0, progress * 100.0))
		);
		let progress_percent_len: usize = 4;

		// How much space do we have?
		let total_len: usize = Display::width() - 5;
		// Brackets around [elapsed], plus two spaces on either side of
		// the bar, and two spaces between the progress numbers.
		let space_len: usize = 2 + 2 + 2 + 2;

		// The bar bits!
		let bar_len: usize = total_len - elapsed_len - progress_num_len - progress_percent_len - space_len;
		let bar_done_len: usize = f64::floor(progress * bar_len as f64) as usize;
		let bar_undone_len = bar_len - bar_done_len;
		let bar_done: String = match bar_done_len {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x]).unwrap_or("".to_string()),
		};
		let bar_undone: String = match bar_undone_len {
			0 => "".to_string(),
			x => String::from_utf8(vec![b'#'; x]).unwrap_or("".to_string()),
		};

		format!(
			"[{}]  {}{}  {}  {}",
			elapsed,
			Colour::Cyan.bold().paint(bar_done),
			Colour::Cyan.dimmed().paint(bar_undone),
			progress_num,
			progress_percent
		)
	}



	// -----------------------------------------------------------------
	// Misc Helpers
	// -----------------------------------------------------------------

	/// Obtain the terminal cli width.
	pub fn width() -> usize {
		match term_size::dimensions() {
			Some((w, _)) => w,
			_ => 0,
		}
	}



	// -----------------------------------------------------------------
	// Arc Helpers
	// -----------------------------------------------------------------

	/// Print Error and Exit.
	pub fn arc_die(display: Arc<Mutex<Display>>, error: FlacaError) {
		let d = display.lock().unwrap();
		d.die(error)
	}

	/// Reset.
	pub fn arc_reset(display: Arc<Mutex<Display>>) {
		let mut d = display.lock().unwrap();
		d.reset();
	}

	/// Watch.
	pub fn arc_watch(display: Arc<Mutex<Display>>, config: Arc<Mutex<Config>>) {
		let mut d = display.lock().unwrap();
		d.watch(config);
	}
}
