/*!
# Core
*/

use crate::config::Config;
use crate::error::FlacaError;
use crate::format;
use crate::image::{ImageApp, ImageKind};
use crate::log::{LogEntry, LogEntryKind, LogTimer};
use crate::reporter::Reporter;
use crate::result::FlacaResult;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};



#[derive(Debug)]
/// The Core.
pub struct Core {
	/// The Configuration.
	config: Arc<Mutex<Config>>,
}

impl Default for Core {
	/// Default.
	fn default() -> Core {
		Core {
			config: Arc::new(Mutex::new(Config::default())),
		}
	}
}

impl Core {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// New.
	pub fn new(config: Config) -> Core {
		Core {
			config: Arc::new(Mutex::new(config)),
		}
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Config.
	pub fn config(&self) -> Arc<Mutex<Config>> {
		self.config.clone()
	}

	/// Reporter.
	pub fn reporter(&self) -> Arc<Mutex<Reporter>> {
		let ptr = self.config.clone();
		let c = ptr.lock().unwrap();
		c.reporter()
	}



	// -----------------------------------------------------------------
	// Setters
	// -----------------------------------------------------------------

	/// Set Config.
	pub fn set_config(&self, config: Config) {
		let ptr = self.config.clone();
		let mut c = ptr.lock().unwrap();
		*c = config;
	}



	// -----------------------------------------------------------------
	// Compression!
	// -----------------------------------------------------------------

	/// Run Compression!
	///
	/// Try to losslessly compress one or more paths.
	pub fn run(&self, paths: &Vec<PathBuf>) -> Result<Vec<FlacaResult>, FlacaError> {
		// Don't double-run.
		if true == Config::arc_is_running(self.config.clone()) {
			return Err(FlacaError::AlreadyRunning);
		}
		// Or obviously bad paths.
		else if true == paths.is_empty() {
			return Err(FlacaError::NoImages);
		}

		let reporter = Config::arc_reporter(self.config.clone());

		// Set up the reporter.
		Reporter::arc_start(reporter.clone());

		// Start a timer.
		let mut timer: LogTimer = LogTimer::default();
		Reporter::arc_push(
			reporter.clone(),
			timer.start(LogEntryKind::Notice, "Flaca", None)
		);

		// Parse the paths.
		let (jpegs, pngs) = self._run_parse_paths(&paths)?;
		let jpegs_len: usize = jpegs.len();
		let pngs_len: usize = pngs.len();

		// Update the reporter totals now that we have them.
		Reporter::arc_set_total(reporter.clone(), jpegs_len + pngs_len);

		// Hold our results.
		let results: Vec<FlacaResult> = self._run_queues(&jpegs, &pngs);

		// Did we save anything?
		let saved: usize = results.iter()
			.map(|ref x| x.saved())
			.sum();

		// Send the final log.
		Reporter::arc_push(reporter.clone(), timer.stop(Some(saved)));

		// Break it back down.
		Reporter::arc_stop(reporter.clone());

		// Return the results!
		Ok(results)
	}

	/// Parse Image Paths.
	fn _run_parse_paths(&self, paths: &Vec<PathBuf>) -> Result<(Vec<PathBuf>, Vec<PathBuf>), FlacaError> {
		let reporter = Config::arc_reporter(self.config.clone());
		let paths: Vec<PathBuf> = format::path::walk(&paths)?;
		if true == paths.is_empty() {
			Reporter::arc_push(
				reporter.clone(),
				LogEntry::from_error(FlacaError::NoImages, None)
			);

			return Err(FlacaError::NoImages);
		}

		let mut jpegs: Vec<PathBuf> = Vec::new();
		let mut pngs: Vec<PathBuf> = Vec::new();
		for path in paths.as_slice() {
			match format::path::image_kind(&path, false) {
				ImageKind::Jpeg => jpegs.push(path.to_path_buf()),
				ImageKind::Png => pngs.push(path.to_path_buf()),
				_ => {},
			}
		}

		// Let's force them empty if we're missing workers.
		if
			false == jpegs.is_empty() &&
			false == Config::arc_has_image_apps(self.config.clone(), ImageKind::Jpeg)
		{
			jpegs.clear();
			jpegs.shrink_to_fit();
		}

		if
			false == pngs.is_empty() &&
			false == Config::arc_has_image_apps(self.config.clone(), ImageKind::Png)
		{
			pngs.clear();
			pngs.shrink_to_fit();
		}

		// If we have nothing, return sadness.
		if true == jpegs.is_empty() && true == pngs.is_empty() {
			Reporter::arc_push(
				reporter.clone(),
				LogEntry::from_error(FlacaError::NoImages, None)
			);

			return Err(FlacaError::NoImages);
		}

		Ok((jpegs, pngs))
	}

	/// Run Queues.
	fn _run_queues(&self, jpegs: &Vec<PathBuf>, pngs: &Vec<PathBuf>) -> Vec<FlacaResult> {
		// If JPEGs are empty, we can just worry about PNGs.
		if true == jpegs.is_empty() {
			return Self::_run_queue(
				self.config.clone(),
				&pngs,
				ImageKind::Png
			);
		}
		// Or if PNGs are empty, we can just worry about JPEGs.
		else if true == pngs.is_empty() {
			return Self::_run_queue(
				self.config.clone(),
				&jpegs,
				ImageKind::Jpeg
			);
		}

		let c1 = self.config.clone();
		let c2 = self.config.clone();

		let jpeg_handle = || Self::_run_queue(
			c1,
			&jpegs,
			ImageKind::Jpeg
		);
		let png_handle = || Self::_run_queue(
			c2,
			&pngs,
			ImageKind::Png
		);

		let (mut r_jpeg, r_png) = rayon::join(jpeg_handle, png_handle);

		// Send cummulative results back.
		r_jpeg.extend(r_png);
		r_jpeg
	}

	/// Run Queue.
	fn _run_queue(
		config: Arc<Mutex<Config>>,
		queue: &Vec<PathBuf>,
		kind: ImageKind
	) -> Vec<FlacaResult> {
		let reporter = Config::arc_reporter(config.clone());
		let queue_len: usize = queue.len();
		let apps: Vec<ImageApp> = Config::arc_image_apps(config.clone(), kind).unwrap();

		// Let's gather a bit more information for debugging purposes,
		// but only if the reporting level wants it.
		if 4 == Reporter::arc_level(reporter.clone()) {
			let apps_nice: Vec<String> = apps.iter().map(|ref x| x.to_string()).collect();

			// Debug message: how many images of this type are there?
			Reporter::arc_push(reporter.clone(), LogEntry::new(
				LogEntryKind::Debug,
				format!(
					"Trying to compress {} with {}.",
					format::grammar::inflect(
						queue_len,
						format!("{} image", kind),
						format!("{} images", kind),
					),
					format::grammar::oxford_join(apps_nice, "and"),
				),
				None,
			));
		}

		// Hold the results.
		let mut out: Vec<FlacaResult> = Vec::new();

		// Loop!
		for path in queue.as_slice() {
			if let Ok(r) = Self::_run_image(reporter.clone(), &path, &apps, kind) {
				out.push(r);
			}

			// Bump the progress.
			Reporter::arc_inc_done(reporter.clone());
		}

		// Done!
		out
	}

	/// Run Single Image.
	fn _run_image<P> (
		reporter: Arc<Mutex<Reporter>>,
		path: P,
		apps: &Vec<ImageApp>,
		kind: ImageKind
	) -> Result<FlacaResult, FlacaError>
	where P: AsRef<Path> {
		// Note our starting size.
		let start_size: usize = format::path::file_size(&path);
		if 0 == start_size {
			Reporter::arc_push(
				reporter.clone(),
				LogEntry::from_error(FlacaError::InvalidPath, Some(path.as_ref().to_path_buf()))
			);
			return Err(FlacaError::InvalidPath);
		}

		// Start a timer for the image as a whole.
		let mut timer: LogTimer = LogTimer::default();
		Reporter::arc_push(
			reporter.clone(),
			timer.start(LogEntryKind::Notice, "image compression", Some(path.as_ref().to_path_buf()))
		);

		// Start a result.
		let mut out: FlacaResult = FlacaResult {
			path: path.as_ref().to_path_buf(),
			kind: kind,
			duration: 0.0,
			size: (start_size, start_size),
		};

		// Keep track of dry-runness.
		let dry_run: bool = Reporter::arc_dry_run(reporter.clone());

		// For dry runs, just clone the image to a new location and mess
		// with that.
		let path: PathBuf = match dry_run {
			true => format::path::tmp_copy_file(&path)?,
			false => path.as_ref().to_path_buf(),
		};

		// Do this for each and every app.
		for app in apps.as_slice() {
			// Start a timer for the specific app run.
			let mut timer2: LogTimer = LogTimer::default();
			Reporter::arc_push(
				reporter.clone(),
				timer2.start(LogEntryKind::Debug, app.to_string(), Some(out.path.clone()))
			);

			let before: usize = format::path::file_size(&path);
			match app.compress(&path) {
				Ok(_) => {
					let after: usize = format::path::file_size(&path);
					let diff: usize = match 0 < after && after < before {
						true => before - after,
						false => 0,
					};

					// An ending log.
					Reporter::arc_push(reporter.clone(), timer2.stop(Some(diff)));
				},
				Err(e) => {
					Reporter::arc_push(
						reporter.clone(),
						LogEntry::from_error(e, Some(out.path.clone()))
					);
				},
			}
		}

		// Our ending size.
		let end_size: usize = format::path::file_size(&path);
		out.size = (start_size, end_size);
		out.duration = timer.elapsed();

		// If this was a dry run, we can delete the temporary file.
		if true == dry_run && path.exists() {
			if let Err(_) = format::path::delete_file(&path) {}
		}

		// An ending log.
		Reporter::arc_push(reporter.clone(), timer.stop(Some(out.saved())));

		Ok(out)
	}
}



#[cfg(test)]
mod tests {
	use super::*;



	#[test]
	#[ignore]
	/// Test Reporter Operations.
	fn test_core_ops() {
		let config: Config = Config::default();

		// We don't want to make real changes.
		config.set_dry_run(true);
		assert!(config.dry_run());

		// Make sure reporting is set to 3.
		config.set_level(3);
		assert_eq!(config.level(), 3);

		// Get a Core going!
		let core: Core = Core::new(config);
		let config: Arc<Mutex<Config>> = core.config();

		let has_jpegs: bool = Config::arc_has_image_apps(config.clone(), ImageKind::Jpeg);
		let has_pngs: bool = Config::arc_has_image_apps(config.clone(), ImageKind::Png);

		// Test the internal app validation logic with the default paths
		// starting with JPEG apps.
		match Config::arc_image_apps(config.clone(), ImageKind::Jpeg) {
			Some(_) => assert!(has_jpegs),
			_ => assert_eq!(has_jpegs, false),
		}

		// And the same for PNG apps.
		match Config::arc_image_apps(config.clone(), ImageKind::Png) {
			Some(_) => assert!(has_pngs),
			_ => assert_eq!(has_pngs, false),
		}

		let paths: Vec<PathBuf> = vec![format::path::abs_pathbuf("./tests/assets")];
		match core.run(&paths) {
			Ok(r) => {
				let expected_len: usize = if has_jpegs && has_pngs {
					12
				}
				else if has_jpegs || has_pngs {
					6
				}
				else {
					0
				};

				assert_eq!(r.len(), expected_len);
			},
			Err(e) => if has_jpegs || has_pngs {
				// If we have JPEG or PNG apps, we shouldn't have an
				// error!
				panic!("{}", e);
			},
		}
	}
}
