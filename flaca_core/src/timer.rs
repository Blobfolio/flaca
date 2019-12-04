/*!
# Timer
*/

use crate::alert::{Alert, AlertKind};
use crate::error::Error;
use crate::format::{self, FormatKind};
use std::path::PathBuf;
use std::time::Instant;


#[derive(Debug, Clone)]
/// Timer.
pub struct Timer {
	name: String,
	time: Option<Instant>,
}

impl Default for Timer {
	/// Default.
	fn default() -> Timer {
		Timer {
			name: "".to_string(),
			time: None,
		}
	}
}

impl Timer {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// New.
	pub fn new<S> (name: S) -> Timer
	where S: Into<String> {
		Timer {
			name: name.into(),
			..Timer::default()
		}
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Elapsed.
	pub fn elapsed(&self) -> f64 {
		match self.time {
			Some(ref t) => t.elapsed().as_millis() as f64 / 1000.0,
			_ => 0.0,
		}
	}



	// -----------------------------------------------------------------
	// Evaluation
	// -----------------------------------------------------------------

	/// Is Running?
	pub fn is_running(&self) -> bool {
		self.time.is_some()
	}



	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Start.
	pub fn start(
		&mut self,
		kind: AlertKind,
		path: Option<PathBuf>,
	) -> Alert {
		self.time.replace(Instant::now());

		Alert::new(
			kind,
			format!("Started {}.", self.name.clone()),
			path,
			None,
			None,
		)
	}

	/// Stop.
	pub fn stop(
		&mut self,
		mut kind: AlertKind,
		path: Option<PathBuf>,
		size: Option<(usize, usize)>,
	) -> Alert {
		// Can't stop before we've started.
		if self.time.is_none() {
			return Alert::from(Error::NullAlert);
		}

		// Upgrade notices to successes?
		if AlertKind::Notice == kind {
			if let Some((ref b, ref a)) = size {
				if 0 < *a && a < b {
					kind = AlertKind::Success;
				}
			}
		}

		// Improve the message using elapsed.
		let nice_elapsed: String = format::time::human_elapsed(self.time.unwrap_or(Instant::now()), FormatKind::Long);
		let msg: String = match "0 seconds" == nice_elapsed {
			true => format!("Finished {}.", self.name.clone()),
			false => format!("Finished {} in {}.", self.name.clone(), nice_elapsed),
		};

		// Grab the elapsed time.
		let elapsed = self.elapsed();

		// Let's stop the clock.
		self.time = None;

		Alert::new(
			kind,
			&msg,
			path,
			Some(elapsed),
			size,
		)
	}
}
