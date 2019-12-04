/*!
# Timer
*/

use crate::alert::{Alert, AlertKind};
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
		kind: AlertKind,
		path: Option<PathBuf>,
		size: Option<(usize, usize)>,
	) -> Alert {
		let elapsed = self.elapsed();
		self.time = None;

		Alert::new(
			kind,
			format!("Started {}.", self.name.clone()),
			path,
			Some(elapsed),
			size,
		)
	}
}
