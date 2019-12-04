/*!
# Alert
*/

use chrono::prelude::*;
use crate::error::Error;
use std::fmt;
use std::path::PathBuf;



#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// Alert Kind.
pub enum AlertKind {
	/// Debug.
	Debug,
	/// Notice.
	Notice,
	/// Warning.
	Warning,
	/// Error.
	Error,
	/// Success.
	Success,
	/// A miscellaneous pass-through type.
	Other,
}

impl fmt::Display for AlertKind {
	#[inline]
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.prefix())
	}
}

impl Into<String> for AlertKind {
	/// Into String.
	fn into(self) -> String {
		match self {
			Self::Debug => "Debug",
			Self::Notice => "Notice",
			Self::Warning => "Warning",
			Self::Error => "Error",
			Self::Success => "Success",
			Self::Other => "",
		}.to_string()
	}
}

impl AlertKind {
	/// Reporting Level.
	pub fn level(&self) -> usize {
		match *self {
			Self::Debug => 4,
			Self::Notice => 3,
			Self::Warning => 2,
			Self::Error => 1,
			Self::Success => 1,
			Self::Other => 1,
		}
	}

	/// Prefix.
	pub fn prefix(&self) -> String {
		match *self {
			Self::Other => "".to_string(),
			_ => format!("{}: ", self)
		}
	}
}



#[derive(Debug, Clone)]
/// Alert.
pub struct Alert {
	date: DateTime<Local>,
	kind: AlertKind,
	msg: String,
	path: Option<PathBuf>,
	elapsed: Option<f64>,
	size: Option<(usize, usize)>,
}

impl Default for Alert {
	/// Default.
	fn default() -> Alert {
		Alert {
			date: Local::now(),
			kind: AlertKind::Debug,
			msg: "".to_string(),
			path: None,
			elapsed: None,
			size: None,
		}
	}
}

/// From Error.
impl From<Error> for Alert {
	fn from(error: Error) -> Alert {
		Alert {
			kind: AlertKind::Error,
			msg: format!("{}", &error),
			..Alert::default()
		}
	}
}

impl Alert {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// New.
	pub fn new<S> (
		kind: AlertKind,
		msg: S,
		path: Option<PathBuf>,
		elapsed: Option<f64>,
		size: Option<(usize, usize)>,
	) -> Alert
	where S: Into<String> {
		Alert {
			kind: kind,
			msg: msg.into(),
			path: path,
			elapsed: elapsed,
			size: size,
			..Alert::default()
		}
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Date.
	pub fn date(&self) -> DateTime<Local> {
		self.date
	}

	/// Elapsed.
	pub fn elapsed(&self) -> Option<f64> {
		match self.elapsed {
			Some(ref e) => Some(e.clone()),
			_ => None,
		}
	}

	/// Kind.
	pub fn kind(&self) -> AlertKind {
		self.kind
	}

	/// Level.
	pub fn level(&self) -> usize {
		self.kind.level()
	}

	/// Path.
	pub fn path(&self) -> Option<PathBuf> {
		match self.path {
			Some(ref p) => Some(p.clone()),
			_ => None,
		}
	}

	/// Size.
	pub fn size(&self) -> Option<(usize, usize)> {
		match self.size {
			Some((ref b, ref a)) => Some((b.clone(), a.clone())),
			_ => None,
		}
	}
}
