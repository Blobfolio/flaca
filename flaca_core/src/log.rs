/*!
# Logging

We have a lot of events to keep track of!
*/

use chrono::prelude::*;
use crate::error::FlacaError;
use crate::format as Format;
use Format::FormatKind;
use std::fmt;
use std::path::PathBuf;
use std::time::Instant;



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Log Entry.
pub enum LogEntryKind {
	/// Debug message.
	Debug,
	/// Success message.
	Success,
	/// A general message.
	Notice,
	/// A warning message.
	Warning,
	/// An error message.
	Error,
	/// A passthrough message.
	Other,
}

impl fmt::Display for LogEntryKind {
	#[inline]
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.prefix())
	}
}

impl LogEntryKind {
	/// Verbosity Level.
	pub fn level(&self) -> usize {
		match *self {
			Self::Debug => 4,
			Self::Success => 1,
			Self::Notice => 3,
			Self::Warning => 2,
			Self::Error => 1,
			Self::Other => 1,
		}
	}

	/// Prefix.
	pub fn prefix(&self) -> String {
		match *self {
			Self::Other => "".to_string(),
			_ => format!("{}: ", self.to_string()),
		}
	}

	/// Prefix.
	pub fn prefix_len(&self) -> usize {
		self.prefix().len()
	}

	/// To String.
	pub fn to_string(&self) -> String {
		match *self {
			Self::Debug => "Debug",
			Self::Success => "Success",
			Self::Notice => "Notice",
			Self::Warning => "Warning",
			Self::Error => "Error",
			Self::Other => "",
		}.to_string()
	}
}



#[derive(Debug, Clone, PartialEq)]
/// A Log Entry.
///
/// Log entries are intended to be simple, syslog-like statements like
/// an ImageApp starting, stopping, etc.
pub struct LogEntry {
	/// The date the message was set.
	pub date: DateTime<Local>,
	/// The kind of message.
	pub kind: LogEntryKind,
	/// The message.
	pub msg: String,
	/// The time taken, if applicable.
	pub elapsed: Option<f64>,
	/// A path, if applicable.
	pub path: Option<PathBuf>,
	/// Savings to report, if applicable.
	pub saved: Option<usize>,
}

impl Default for LogEntry {
	/// Default.
	fn default() -> LogEntry {
		LogEntry {
			date: Local::now(),
			kind: LogEntryKind::Debug,
			msg: "".to_string(),
			elapsed: None,
			path: None,
			saved: None,
		}
	}
}

impl fmt::Display for LogEntry {
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "")
	}
}

impl LogEntry {
	/// New.
	pub fn new<S> (kind: LogEntryKind, msg: S, path: Option<PathBuf>) -> Self
	where S: Into<String> {
		LogEntry {
			kind: kind,
			msg: msg.into(),
			path: path,
			..LogEntry::default()
		}
	}

	/// From Error.
	pub fn from_error<E> (error: E, path: Option<PathBuf>) -> Self
	where E: Into<FlacaError> {
		LogEntry {
			kind: LogEntryKind::Error,
			msg: error.into().as_string(),
			path: path,
			..LogEntry::default()
		}
	}

	/// Level.
	pub fn level(&self) -> usize {
		self.kind.level()
	}
}



#[derive(Debug, Clone, PartialEq)]
/// Log Timer.
///
/// Most events have both a start and end log entry, the latter
/// indicating some amount of time having passed. The timer can be used
/// to help put those entries together quickly.
pub struct LogTimer {
	time: Instant,
	kind: LogEntryKind,
	name: String,
	path: Option<PathBuf>,
	saved: Option<usize>,
}

impl Default for LogTimer {
	/// Default.
	fn default() -> LogTimer {
		LogTimer {
			time: Instant::now(),
			kind: LogEntryKind::Notice,
			name: "".to_string(),
			path: None,
			saved: None,
		}
	}
}

impl LogTimer {
	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Elapsed.
	pub fn elapsed(&self) -> f64 {
		self.time.elapsed().as_millis() as f64 / 1000.0
	}



	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Start.
	///
	/// Generate a starting log entry.
	pub fn start<S> (
		&mut self,
		kind: LogEntryKind,
		name: S,
		path: Option<PathBuf>
	) -> LogEntry
	where S: Into<String> {
		self.kind = kind;
		self.name = name.into();
		self.path = path;

		LogEntry {
			kind: self.kind,
			msg: format!("Started {}.", self.name),
			path: self.path.clone(),
			..LogEntry::default()
		}
	}

	/// Stop.
	///
	/// Generate a stopping log entry.
	pub fn stop(&mut self, saved: Option<usize>) -> LogEntry {
		// Update the amount saved.
		self.saved = saved;

		let kind: LogEntryKind = match self.kind {
			// If the original is a notice, we can upgrade it to a
			// success if we accomplished something.
			LogEntryKind::Notice => match self.saved {
				Some(saved) => match saved {
					0 => LogEntryKind::Notice,
					_ => LogEntryKind::Success,
				},
				_ => LogEntryKind::Notice,
			},
			_ => self.kind,
		};

		// We don't need to report completion time if it is zero
		// seconds.
		let elapsed: String = Format::time::human_elapsed(self.time, FormatKind::Long);
		let msg: String = if elapsed == "0 seconds" {
			format!("Finished {}.", self.name)
		}
		else {
			format!("Finished {} in {}.", self.name, elapsed)
		};

		LogEntry {
			kind: kind,
			msg: msg,
			path: self.path.clone(),
			saved: self.saved.clone(),
			..LogEntry::default()
		}
	}
}
