/*!
# Reporter

The Reporter can best be thought of as a detached store of state that
can be intermittently accessed across threads (while the Core itself is
tied up in compression business).
*/

use crate::log::LogEntry;
use crossbeam_channel::Sender;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Debug)]
/// Reporter.
pub struct Reporter {
	/// Currently Compressing?
	running: AtomicBool,
	/// Dry Run?
	dry_run: AtomicBool,
	/// Reporting Level.
	level: AtomicUsize,
	/// Jobs Completed.
	done: AtomicUsize,
	/// Total Jobs.
	total: AtomicUsize,
	/// Log Entries.
	sender: Arc<Mutex<Option<Sender<LogEntry>>>>,
}

impl Default for Reporter {
	/// Default.
	fn default() -> Reporter {
		Reporter {
			running: AtomicBool::new(false),
			dry_run: AtomicBool::new(false),
			level: AtomicUsize::new(3),
			done: AtomicUsize::new(0),
			total: AtomicUsize::new(0),
			sender: Arc::new(Mutex::new(None)),
		}
	}
}

impl Reporter {
	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Jobs Completed.
	///
	/// Return the total number of jobs so far completed.
	pub fn done(&self) -> usize {
		self.done.load(Ordering::Relaxed)
	}

	/// Dry Run.
	///
	/// Return whether or not this is a dry run.
	pub fn dry_run(&self) -> bool {
		self.dry_run.load(Ordering::Relaxed)
	}

	/// Has Sender.
	pub fn has_sender(&self) -> bool {
		let ptr = self.sender.clone();
		let l = ptr.lock().unwrap();
		l.is_some()
	}

	/// Reporting Level.
	///
	/// Return the reporting level being used.
	pub fn level(&self) -> usize {
		self.level.load(Ordering::Relaxed)
	}

	/// Currently Running?
	///
	/// Return whether or not compression operations are currently
	/// underway.
	pub fn running(&self) -> bool {
		self.running.load(Ordering::Relaxed)
	}

	/// Total Jobs.
	///
	/// Return the total number of jobs being processed.
	pub fn total(&self) -> usize {
		self.total.load(Ordering::Relaxed)
	}



	// -----------------------------------------------------------------
	// Setters
	// -----------------------------------------------------------------

	/// Set Jobs Completed.
	pub fn set_done(&self, done: usize) {
		let total: usize = self.total();

		match total >= done {
			true => self.done.store(done, Ordering::Relaxed),
			false => self.done.store(total, Ordering::Relaxed),
		}
	}

	/// Set Dry Run.
	pub fn set_dry_run(&self, dry_run: bool) {
		self.dry_run.store(dry_run, Ordering::Relaxed);
	}

	/// Set Reporting Level.
	pub fn set_level(&self, level: usize) {
		match 4 >= level  {
			true => self.level.store(level, Ordering::Relaxed),
			false => self.level.store(4, Ordering::Relaxed)
		}
	}

	/// Set Currently Running.
	pub fn set_running(&self, running: bool) {
		self.running.store(running, Ordering::Relaxed)
	}

	/// Set Sender.
	pub fn set_sender(&self, sender: Option<Sender<LogEntry>>) {
		let ptr = self.sender.clone();
		let mut l = ptr.lock().unwrap();
		*l = sender;
	}

	/// Set Total Jobs.
	pub fn set_total(&self, total: usize) {
		self.total.store(total, Ordering::Relaxed)
	}



	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Increment Completed Jobs.
	///
	/// This method increases the completed jobs count by one. If and
	/// when the number of completed jobs reaches the total number of
	/// jobs, this method ceases to take any action.
	pub fn inc_done(&self) {
		if self.done.load(Ordering::Relaxed) < self.total.load(Ordering::Relaxed) {
			self.done.fetch_add(1, Ordering::SeqCst);
		}
		else {
			self.done.store(self.total(), Ordering::Relaxed);
		}
	}

	/// Push Log Entry.
	///
	/// Add a new entry to the log.
	pub fn push(&self, entry: LogEntry) {
		if self.level() >= entry.level() {
			let ptr = self.sender.clone();
			let l = ptr.lock().unwrap();
			if let Some(s) = &*l {
				s.send(entry).unwrap();
			}
		}
	}

	/// Start.
	pub fn start(&self) {
		self.set_done(0);
		self.set_total(0);
		self.set_running(true);
	}

	/// Stop.
	pub fn stop(&self) {
		self.set_running(false);

		// Disconnect the sender.
		let ptr = self.sender.clone();
		let mut l = ptr.lock().unwrap();
		if let Some(s) = &l.take() {
			drop(s);
		}
	}



	// -----------------------------------------------------------------
	// Arc Wrappers
	// -----------------------------------------------------------------

	/// Jobs Completed (Arc).
	pub fn arc_done(reporter: Arc<Mutex<Reporter>>) -> usize {
		let r = reporter.lock().unwrap();
		r.done()
	}

	/// Dry Run? (Arc).
	pub fn arc_dry_run(reporter: Arc<Mutex<Reporter>>) -> bool {
		let r = reporter.lock().unwrap();
		r.dry_run()
	}

	/// Has Sender (Arc).
	pub fn arc_has_sender(reporter: Arc<Mutex<Reporter>>) -> bool {
		let r = reporter.lock().unwrap();
		r.has_sender()
	}

	/// Reporting Level (Arc).
	pub fn arc_level(reporter: Arc<Mutex<Reporter>>) -> usize {
		let r = reporter.lock().unwrap();
		r.level()
	}

	/// Currently Running (Arc)?
	pub fn arc_running(reporter: Arc<Mutex<Reporter>>) -> bool {
		let r = reporter.lock().unwrap();
		r.running()
	}

	/// Total Jobs (Arc).
	pub fn arc_total(reporter: Arc<Mutex<Reporter>>) -> usize {
		let r = reporter.lock().unwrap();
		r.total()
	}

	/// Increment Completed Jobs Count (Arc).
	pub fn arc_inc_done(reporter: Arc<Mutex<Reporter>>) {
		let r = reporter.lock().unwrap();
		r.inc_done();
	}

	/// Push Log Entry (Arc).
	pub fn arc_push(reporter: Arc<Mutex<Reporter>>, entry: LogEntry) {
		let r = reporter.lock().unwrap();
		r.push(entry);
	}

	/// Start (Arc).
	pub fn arc_start(reporter: Arc<Mutex<Reporter>>) {
		let r = reporter.lock().unwrap();
		r.start();
	}

	/// Stop (Arc).
	pub fn arc_stop(reporter: Arc<Mutex<Reporter>>) {
		let r = reporter.lock().unwrap();
		r.stop();
	}

	/// Set Completed Jobs (Arc).
	pub fn arc_set_done(reporter: Arc<Mutex<Reporter>>, done: usize) {
		let r = reporter.lock().unwrap();
		r.set_done(done);
	}

	/// Set Dry Run (Arc).
	pub fn arc_set_dry_run(reporter: Arc<Mutex<Reporter>>, dry_run: bool) {
		let r = reporter.lock().unwrap();
		r.set_dry_run(dry_run);
	}

	/// Set Reporting Level (Arc).
	pub fn arc_set_level(reporter: Arc<Mutex<Reporter>>, level: usize) {
		let r = reporter.lock().unwrap();
		r.set_level(level);
	}

	/// Set Sender.
	pub fn arc_set_sender(reporter: Arc<Mutex<Reporter>>, sender: Option<Sender<LogEntry>>) {
		let r = reporter.lock().unwrap();
		r.set_sender(sender);
	}

	/// Set Total Jobs (Arc).
	pub fn arc_set_total(reporter: Arc<Mutex<Reporter>>, total: usize) {
		let r = reporter.lock().unwrap();
		r.set_total(total);
	}
}



#[cfg(test)]
mod tests {
	use super::*;
	use crate::log::LogEntryKind;
	use crossbeam_channel::{TryRecvError, unbounded};



	#[test]
	/// Test Reporter Operations.
	fn test_reporter_ops() {
		// Create an Arc Reporter so we can test both wrappers and what
		// they wrap.
		let reporter: Arc<Mutex<Reporter>> = Arc::new(Mutex::new(Reporter::default()));

		// Start with a reset, just in case.
		let (tx, rx) = unbounded();
		Reporter::arc_set_sender(reporter.clone(), Some(tx.clone()));
		Reporter::arc_start(reporter.clone());

		// Job and running should be off.
		assert_eq!(Reporter::arc_done(reporter.clone()), 0);
		assert_eq!(Reporter::arc_total(reporter.clone()), 0);
		assert_eq!(Reporter::arc_running(reporter.clone()), true);

		// Check the levels.
		Reporter::arc_set_level(reporter.clone(), 5);
		assert_eq!(Reporter::arc_level(reporter.clone()), 4);
		Reporter::arc_set_level(reporter.clone(), 3);
		assert_eq!(Reporter::arc_level(reporter.clone()), 3);

		// Check dry run.
		Reporter::arc_set_dry_run(reporter.clone(), true);
		assert_eq!(Reporter::arc_dry_run(reporter.clone()), true);
		Reporter::arc_set_dry_run(reporter.clone(), false);
		assert_eq!(Reporter::arc_dry_run(reporter.clone()), false);

		// Mess with the totals a bit.
		Reporter::arc_set_total(reporter.clone(), 10);
		assert_eq!(Reporter::arc_total(reporter.clone()), 10);
		Reporter::arc_set_done(reporter.clone(), 8);
		assert_eq!(Reporter::arc_done(reporter.clone()), 8);

		// Test incrementing.
		Reporter::arc_inc_done(reporter.clone());
		assert_eq!(Reporter::arc_done(reporter.clone()), 9);
		Reporter::arc_inc_done(reporter.clone());
		assert_eq!(Reporter::arc_done(reporter.clone()), 10);

		// A further attempt to increment shouldn't do anything.
		Reporter::arc_inc_done(reporter.clone());
		assert_eq!(Reporter::arc_done(reporter.clone()), 10);

		// Let's make some log entries.
		let log1: LogEntry = LogEntry::new(
			LogEntryKind::Debug,
			"Message One",
			None
		);
		let log2: LogEntry = LogEntry::new(
			LogEntryKind::Notice,
			"Message Two",
			None
		);
		let log3: LogEntry = LogEntry::new(
			LogEntryKind::Warning,
			"Message Three",
			None
		);

		// Add the first entry. It should be rejected because of the
		// set level.
		Reporter::arc_push(reporter.clone(), log1.clone());
		assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

		// The others should add just fine, though.
		Reporter::arc_push(reporter.clone(), log2.clone());
		Reporter::arc_push(reporter.clone(), log3.clone());

		assert_eq!(rx.try_recv(), Ok(log2));
		assert_eq!(rx.try_recv(), Ok(log3));
		assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));

		// Let's stop it.
		assert!(Reporter::arc_has_sender(reporter.clone()));
		Reporter::arc_stop(reporter.clone());
		assert_eq!(Reporter::arc_running(reporter.clone()), false);
		assert_eq!(rx.try_recv(), Err(TryRecvError::Empty));
		assert_eq!(Reporter::arc_has_sender(reporter.clone()), false);
	}
}
