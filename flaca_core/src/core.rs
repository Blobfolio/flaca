/*!
# Core
*/

use crate::alert::{Alert, AlertKind};
use crate::error::Error;
use crate::format;
use crate::image::{App, ImageKind};
use crate::paths::{PathDisplay, PathIO, PathProps, PathVec};
use crate::timer::Timer;
use crossbeam_channel::Sender;
use serde::de::{Deserialize, Deserializer, Visitor, MapAccess};
use serde::ser::{Serialize, Serializer, SerializeStruct};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Debug)]
/// The Core.
pub struct Core {
	/// The Configuration.
	inner: Arc<Mutex<CoreState>>,
}

impl Default for Core {
	/// Default.
	fn default() -> Core {
		Core {
			inner: Arc::new(Mutex::new(CoreState::default())),
		}
	}
}

impl Core {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// New.
	pub fn new(settings: CoreSettings) -> Core {
		Core {
			inner: Arc::new(Mutex::new(CoreState::from(settings))),
		}
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// State.
	pub fn state(&self) -> Arc<Mutex<CoreState>> {
		self.inner.clone()
	}

	// -----------------------------------------------------------------
	// Compression!
	// -----------------------------------------------------------------

	/// Run Compression!
	///
	/// Try to losslessly compress one or more images.
	pub fn run(&self, paths: &Vec<PathBuf>) -> Result<(), Error> {
		// Don't double-run.
		if true == CoreState::arc_is_running(self.inner.clone()) {
			return Err(Error::DoubleRun);
		}
		// And abort if there are already zero paths.
		else if true == paths.is_empty() {
			return Err(Error::NoImages);
		}

		// Start yer engines.
		CoreState::arc_start(self.inner.clone());

		// Parse the paths.
		let (jpegs, pngs) = self._run_parse_paths(&paths)?;
		let jpegs_len: usize = jpegs.len();
		let pngs_len: usize = pngs.len();

		// Calculate the original size.
		let before: usize = jpegs.flaca_file_sizes() + pngs.flaca_file_sizes();

		// Update the reporter totals now that we have them.
		CoreState::arc_set_total(self.inner.clone(), jpegs_len + pngs_len);

		// Hold our results.
		let saved: usize = self._run_queues(&jpegs, &pngs);

		// Send the final log.
		let after: usize = before - saved;
		CoreState::arc_stop(self.inner.clone(), before, after);

		// Return the results!
		Ok(())
	}

	/// Parse Image Paths.
	fn _run_parse_paths(&self, paths: &Vec<PathBuf>) -> Result<(Vec<PathBuf>, Vec<PathBuf>), Error> {
		let paths: Vec<PathBuf> = paths.flaca_walk()?;
		if true == paths.is_empty() {
			CoreState::arc_send(
				self.inner.clone(),
				Alert::from(Error::NoImages)
			);

			return Err(Error::NoImages);
		}

		let mut jpegs: Vec<PathBuf> = Vec::new();
		let mut pngs: Vec<PathBuf> = Vec::new();
		for path in paths.as_slice() {
			match path.flaca_image_kind(false) {
				ImageKind::Jpeg => jpegs.push(path.to_path_buf()),
				ImageKind::Png => pngs.push(path.to_path_buf()),
				_ => {},
			}
		}

		// Let's force them empty if we're missing workers.
		if
			false == jpegs.is_empty() &&
			false == CoreState::arc_has_image_apps(self.inner.clone(), ImageKind::Jpeg)
		{
			jpegs.clear();
			jpegs.shrink_to_fit();
		}

		if
			false == pngs.is_empty() &&
			false == CoreState::arc_has_image_apps(self.inner.clone(), ImageKind::Png)
		{
			pngs.clear();
			pngs.shrink_to_fit();
		}

		// If we have nothing, return sadness.
		if true == jpegs.is_empty() && true == pngs.is_empty() {
			CoreState::arc_send(
				self.inner.clone(),
				Alert::from(Error::NoImages)
			);

			return Err(Error::NoImages);
		}

		Ok((jpegs, pngs))
	}

	/// Run Queues.
	fn _run_queues(&self, jpegs: &Vec<PathBuf>, pngs: &Vec<PathBuf>) -> usize {
		// If JPEGs are empty, we can just worry about PNGs.
		if true == jpegs.is_empty() {
			return Self::_run_queue(
				self.inner.clone(),
				&pngs,
				ImageKind::Png
			);
		}
		// Or if PNGs are empty, we can just worry about JPEGs.
		else if true == pngs.is_empty() {
			return Self::_run_queue(
				self.inner.clone(),
				&jpegs,
				ImageKind::Jpeg
			);
		}

		let c1 = self.inner.clone();
		let c2 = self.inner.clone();

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

		let (total1, total2) = rayon::join(jpeg_handle, png_handle);
		return total1 + total2;
	}

	/// Run Queue.
	fn _run_queue(
		state: Arc<Mutex<CoreState>>,
		queue: &Vec<PathBuf>,
		kind: ImageKind
	) -> usize {
		let queue_len: usize = queue.len();
		let apps: Vec<App> = CoreState::arc_image_apps(state.clone(), kind).unwrap();

		// Let's gather a bit more information for debugging purposes,
		// but only if the reporting level wants it.
		if 4 == CoreState::arc_level(state.clone()) {
			let apps_nice: Vec<String> = apps.iter()
				.map(|ref x| x.to_string())
				.collect();

			// Debug message: how many images of this type are there?
			CoreState::arc_send(
				state.clone(),
				Alert::new(
					AlertKind::Debug,
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
					None,
					None
				)
			);
		}

		// Loop!
		let mut saved: usize = 0;
		for path in queue.as_slice() {
			if let Ok(s) = Self::_run_image(state.clone(), &path, &apps) {
				saved = saved + s;
			}
			CoreState::arc_inc_done(state.clone());
		}

		saved
	}

	/// Run Single Image.
	fn _run_image<P> (
		state: Arc<Mutex<CoreState>>,
		path: P,
		apps: &Vec<App>
	) -> Result<usize, Error>
	where P: AsRef<Path> {
		// Note our starting size.
		let start_size: usize = path.as_ref().flaca_file_size();
		if 0 == start_size {
			CoreState::arc_send(
				state.clone(),
				Alert::from(Error::InvalidPath(path.as_ref().flaca_to_string()))
			);

			return Err(Error::InvalidPath(path.as_ref().flaca_to_string()));
		}

		// Start a timer for the image as a whole.
		let mut timer: Timer = Timer::new("image compression");
		CoreState::arc_send(
			state.clone(),
			timer.start(
				AlertKind::Notice,
				Some(path.as_ref().to_path_buf())
			)
		);

		// Start a result.
		let real_path = path.as_ref().to_path_buf();

		// Keep track of dry-runness.
		let dry_run: bool = CoreState::arc_dry_run(state.clone());

		// For dry runs, just clone the image to a new location and mess
		// with that.
		let path: PathBuf = match dry_run {
			true => path.as_ref().flaca_copy_tmp()?,
			false => path.as_ref().to_path_buf(),
		};

		// Do this for each and every app.
		for app in apps.as_slice() {
			// Start a timer for the specific app run.
			let mut timer2: Timer = Timer::new(format!("{}", &app));
			CoreState::arc_send(
				state.clone(),
				timer2.start(AlertKind::Debug, Some(real_path.clone()))
			);

			let before: usize = path.flaca_file_size();
			match app.compress(&path) {
				Ok(_) => {
					let after: usize = path.flaca_file_size();
					CoreState::arc_send(state.clone(), timer2.stop(
						AlertKind::Debug,
						Some(real_path.clone()),
						Some((before, after))
					));
				},
				Err(e) => {
					CoreState::arc_send(
						state.clone(),
						Alert::from(e)
					);
				},
			}
		}

		// Our ending size.
		let end_size: usize = path.flaca_file_size();
		let diff = match 0 < end_size && end_size < start_size {
			true => start_size - end_size,
			false => 0,
		};

		// If this was a dry run, we can delete the temporary file.
		if true == dry_run && path.exists() {
			if let Err(_) = path.flaca_delete_file() {}
		}

		// An ending log.
		CoreState::arc_send(
			state.clone(),
			timer.stop(
				AlertKind::Notice,
				Some(real_path.clone()),
				Some((start_size, end_size))
			)
		);

		Ok(diff)
	}
}



#[derive(Debug, Clone)]
/// Core State.
pub struct CoreState {
	dry_run: Arc<AtomicBool>,
	level: Arc<AtomicUsize>,
	jpegoptim: Arc<Mutex<App>>,
	mozjpeg: Arc<Mutex<App>>,
	oxipng: Arc<Mutex<App>>,
	pngout: Arc<Mutex<App>>,
	zopflipng: Arc<Mutex<App>>,
	timer: Arc<Mutex<Timer>>,
	done: Arc<AtomicUsize>,
	total: Arc<AtomicUsize>,
	sender: Arc<Mutex<Option<Sender<Alert>>>>,
}

impl Default for CoreState {
	/// Default.
	fn default() -> CoreState {
		CoreState {
			dry_run: Arc::new(AtomicBool::new(false)),
			level: Arc::new(AtomicUsize::new(3)),
			jpegoptim: Arc::new(Mutex::new(App::None)),
			mozjpeg: Arc::new(Mutex::new(App::None)),
			oxipng: Arc::new(Mutex::new(App::None)),
			pngout: Arc::new(Mutex::new(App::None)),
			zopflipng: Arc::new(Mutex::new(App::None)),
			timer: Arc::new(Mutex::new(Timer::new("Flaca"))),
			done: Arc::new(AtomicUsize::new(0)),
			total: Arc::new(AtomicUsize::new(0)),
			sender: Arc::new(Mutex::new(None)),
		}
	}
}

impl From<CoreSettings> for CoreState {
	/// From.
	fn from(settings: CoreSettings) -> CoreState {
		CoreState {
			dry_run: Arc::new(AtomicBool::new(settings.dry_run())),
			level: Arc::new(AtomicUsize::new(settings.level())),
			jpegoptim: Arc::new(Mutex::new(settings.jpegoptim())),
			mozjpeg: Arc::new(Mutex::new(settings.mozjpeg())),
			oxipng: Arc::new(Mutex::new(settings.oxipng())),
			pngout: Arc::new(Mutex::new(settings.pngout())),
			zopflipng: Arc::new(Mutex::new(settings.zopflipng())),
			..CoreState::default()
		}
	}
}

impl CoreState {
	// Quick apps.
	core_state_quick_apps!(jpegoptim);
	core_state_quick_apps!(mozjpeg);
	core_state_quick_apps!(oxipng);
	core_state_quick_apps!(pngout);
	core_state_quick_apps!(zopflipng);



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Done.
	pub fn done(&self) -> usize {
		atomicity!(self.done.clone())
	}

	/// Dry Run?
	pub fn dry_run(&self) -> bool {
		atomicity!(self.dry_run.clone())
	}

	/// List of Available Image Apps (By Name).
	pub fn image_app_list(&self) -> Vec<String> {
		let mut out: Vec<String> = Vec::new();

		if App::None != self.jpegoptim() {
			out.push("Jpegoptim".to_string());
		}
		if App::None != self.mozjpeg() {
			out.push("MozJPEG".to_string());
		}
		if App::None != self.oxipng() {
			out.push("Oxipng".to_string());
		}
		if App::None != self.pngout() {
			out.push("Pngout".to_string());
		}
		if App::None != self.zopflipng() {
			out.push("Zopflipng".to_string());
		}

		out
	}

	/// Available Image Apps.
	pub fn image_apps(&self, kind: ImageKind) -> Option<Vec<App>> {
		let mut out: Vec<App> = Vec::new();

		match kind {
			ImageKind::Jpeg => {
				let app = self.jpegoptim();
				if App::None != app {
					out.push(app);
				}

				let app = self.mozjpeg();
				if App::None != app {
					out.push(app);
				}
			},
			ImageKind::Png => {
				let app = self.pngout();
				if App::None != app {
					out.push(app);
				}

				let app = self.oxipng();
				if App::None != app {
					out.push(app);
				}

				let app = self.zopflipng();
				if App::None != app {
					out.push(app);
				}
			},
			_ => return None,
		}

		match out.is_empty() {
			true => None,
			false => Some(out),
		}
	}

	/// Level.
	pub fn level(&self) -> usize {
		atomicity!(self.level.clone())
	}

	/// Progress.
	pub fn progress(&self) -> (usize, usize) {
		(self.done(), self.total())
	}

	/// Sender.
	pub fn sender(&self) -> Option<Sender<Alert>> {
		let ptr = self.sender.clone();
		let s = ptr.lock().unwrap();
		match *s {
			Some(ref s) => Some(s.clone()),
			_ => None,
		}
	}

	/// Total.
	pub fn total(&self) -> usize {
		atomicity!(self.total.clone())
	}



	// -----------------------------------------------------------------
	// Setters
	// -----------------------------------------------------------------

	/// Increment Completed Jobs.
	///
	/// This method increases the completed jobs count by one. If and
	/// when the number of completed jobs reaches the total number of
	/// jobs, this method ceases to take any action.
	pub fn inc_done(&self) {
		let ptr1 = self.done.clone();
		let ptr2 = self.total.clone();

		if ptr1.load(Ordering::Relaxed) < ptr2.load(Ordering::Relaxed) {
			ptr1.fetch_add(1, Ordering::SeqCst);
		}
		else {
			ptr1.store(ptr2.load(Ordering::Relaxed), Ordering::Relaxed);
		}
	}

	/// Done.
	pub fn set_done(&self, done: usize) {
		atomicity!(self.done.clone(), done);
	}

	/// Dry Run?
	pub fn set_dry_run(&self, dry_run: bool) {
		atomicity!(self.dry_run.clone(), dry_run);
	}

	/// Level.
	pub fn set_level(&self, mut level: usize) {
		if 4 < level {
			level = 4;
		}

		atomicity!(self.level.clone(), level);
	}

	/// Total.
	pub fn set_total(&self, total: usize) {
		atomicity!(self.total.clone(), total);
	}



	// -----------------------------------------------------------------
	// Evaluation
	// -----------------------------------------------------------------

	/// Has Apps?
	///
	/// Return whether or not at least one App exists for the image
	/// type.
	pub fn has_image_apps(&self, kind: ImageKind) -> bool {
		match kind {
			ImageKind::Jpeg =>
				App::None != self.jpegoptim() ||
				App::None != self.mozjpeg(),
			ImageKind::Png =>
				App::None != self.oxipng() ||
				App::None != self.pngout() ||
				App::None != self.zopflipng(),
			_ => false,
		}
	}

	/// Is Running.
	pub fn is_running(&self) -> bool {
		let ptr = self.timer.clone();
		let t = ptr.lock().unwrap();
		t.is_running()
	}



	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Open Channel.
	pub fn open_channel(&self, sender: Sender<Alert>) {
		let ptr = self.sender.clone();
		let mut s = ptr.lock().unwrap();
		s.replace(sender.clone());
	}

	/// Send Alert.
	pub fn send(&self, alert: Alert) {
		// Don't push events that are beyond our interest.
		if self.level() < alert.level() {
			return;
		}

		let ptr = self.sender.clone();
		let s = ptr.lock().unwrap();

		if let Some(ref sender) = *s {
			sender.send(alert).unwrap();
		}
	}

	/// Start.
	pub fn start(&self) {
		let ptr = self.timer.clone();
		let mut t = ptr.lock().unwrap();

		self.send(t.start(AlertKind::Notice, None));
	}

	/// Stop.
	pub fn stop(&self, before: usize, after: usize) {
		let ptr = self.timer.clone();
		let mut t = ptr.lock().unwrap();

		self.send(t.stop(AlertKind::Notice, None, Some((before, after))));
	}


	// -----------------------------------------------------------------
	// Arc Wrappers
	// -----------------------------------------------------------------

	/// Done.
	pub fn arc_done(state: Arc<Mutex<CoreState>>) -> usize {
		let c = state.lock().unwrap();
		c.done()
	}

	/// Dry Run?
	pub fn arc_dry_run(state: Arc<Mutex<CoreState>>) -> bool {
		let c = state.lock().unwrap();
		c.dry_run()
	}

	/// List of Available Image Apps (By Name).
	pub fn arc_image_app_list(state: Arc<Mutex<CoreState>>) -> Vec<String> {
		let c = state.lock().unwrap();
		c.image_app_list()
	}

	/// Available Image Apps.
	pub fn arc_image_apps(state: Arc<Mutex<CoreState>>, kind: ImageKind) -> Option<Vec<App>> {
		let c = state.lock().unwrap();
		c.image_apps(kind)
	}

	/// Level.
	pub fn arc_level(state: Arc<Mutex<CoreState>>) -> usize {
		let c = state.lock().unwrap();
		c.level()
	}

	/// Progress.
	pub fn arc_progress(state: Arc<Mutex<CoreState>>) -> (usize, usize) {
		let c = state.lock().unwrap();
		c.progress()
	}

	/// Sender.
	pub fn arc_sender(state: Arc<Mutex<CoreState>>) -> Option<Sender<Alert>> {
		let c = state.lock().unwrap();
		c.sender()
	}

	/// Total.
	pub fn arc_total(state: Arc<Mutex<CoreState>>) -> usize {
		let c = state.lock().unwrap();
		c.total()
	}

	/// Increment Completed Jobs.
	pub fn arc_inc_done(state: Arc<Mutex<CoreState>>) {
		let c = state.lock().unwrap();
		c.inc_done()
	}

	/// Done.
	pub fn arc_set_done(state: Arc<Mutex<CoreState>>, done: usize) {
		let c = state.lock().unwrap();
		c.set_done(done)
	}

	/// Dry Run?
	pub fn arc_set_dry_run(state: Arc<Mutex<CoreState>>, dry_run: bool) {
		let c = state.lock().unwrap();
		c.set_dry_run(dry_run)
	}

	/// Level.
	pub fn arc_set_level(state: Arc<Mutex<CoreState>>, level: usize) {
		let c = state.lock().unwrap();
		c.set_level(level)
	}

	/// Total.
	pub fn arc_set_total(state: Arc<Mutex<CoreState>>, total: usize) {
		let c = state.lock().unwrap();
		c.set_total(total)
	}

	/// Has Apps?
	pub fn arc_has_image_apps(state: Arc<Mutex<CoreState>>, kind: ImageKind) -> bool {
		let c = state.lock().unwrap();
		c.has_image_apps(kind)
	}

	/// Is Running.
	pub fn arc_is_running(state: Arc<Mutex<CoreState>>) -> bool {
		let c = state.lock().unwrap();
		c.is_running()
	}

	/// Open Channel.
	pub fn arc_open_channel(state: Arc<Mutex<CoreState>>, sender: Sender<Alert>) {
		let c = state.lock().unwrap();
		c.open_channel(sender)
	}

	/// Send Alert.
	pub fn arc_send(state: Arc<Mutex<CoreState>>, alert: Alert) {
		let c = state.lock().unwrap();
		c.send(alert)
	}

	/// Start.
	pub fn arc_start(state: Arc<Mutex<CoreState>>) {
		let c = state.lock().unwrap();
		c.start()
	}

	/// Stop.
	pub fn arc_stop(state: Arc<Mutex<CoreState>>, before: usize, after: usize) {
		let c = state.lock().unwrap();
		c.stop(before, after)
	}
}



#[derive(Debug, Clone)]
/// Core Settings.
pub struct CoreSettings {
	/// Dry Run.
	dry_run: bool,
	/// Reporting Level.
	level: usize,
	/// Jpegoptim.
	jpegoptim: App,
	/// MozJPEG.
	mozjpeg: App,
	/// Oxipng.
	oxipng: App,
	/// PNGOUT.
	pngout: App,
	/// Zopflipng.
	zopflipng: App,
}

impl Default for CoreSettings {
	/// Default.
	fn default() -> CoreSettings {
		CoreSettings {
			dry_run: false,
			level: 3,
			jpegoptim: App::find_jpegoptim(),
			mozjpeg: App::find_mozjpeg(),
			oxipng: App::find_oxipng(),
			pngout: App::find_pngout(),
			zopflipng: App::find_zopflipng(),
		}
	}
}

impl Serialize for CoreSettings {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		// 3 is the number of fields in the struct.
		let mut state = serializer.serialize_struct("CoreSettings", 6)?;
		state.serialize_field("level", &self.level())?;
		state.serialize_field("jpegoptim", &self.jpegoptim())?;
		state.serialize_field("mozjpeg", &self.mozjpeg())?;
		state.serialize_field("oxipng", &self.oxipng())?;
		state.serialize_field("pngout", &self.pngout())?;
		state.serialize_field("zopflipng", &self.zopflipng())?;
		state.end()
	}
}

impl<'de> Deserialize<'de> for CoreSettings {
	/// Derialize!
	fn deserialize<D> (deserializer: D) -> Result<CoreSettings, D::Error>
	where D: Deserializer<'de> {
		#[derive(Deserialize)]
		#[serde(field_identifier, rename_all = "lowercase")]
		/// Fields.
		enum Field {
			Level,
			Jpegoptim,
			Mozjpeg,
			Oxipng,
			Pngout,
			Zopflipng
		}

		/// Fields Again.
		const FIELDS: &'static [&'static str] = &[
			"level",
			"jpegoptim",
			"mozjpeg",
			"oxipng",
			"pngout",
			"zopflipng",
		];

		/// The Visitor.
		struct CoreSettingsVisitor;
		impl<'de> Visitor<'de> for CoreSettingsVisitor {
			type Value = CoreSettings;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct CoreSettings")
			}

			fn visit_map<V>(self, mut map: V) -> Result<CoreSettings, V::Error>
			where
				V: MapAccess<'de>,
			{
				let mut out: CoreSettings = CoreSettings::default();

				while let Some(key) = map.next_key()? {
					match key {
						Field::Level => {
							if let Ok(level) = map.next_value() {
								let level: usize = level;
								out.set_level(level);
							}
						},
						Field::Jpegoptim => {
							if let Ok(app) = map.next_value() {
								let app: App = app;
								out.set_jpegoptim(app);
							}
						},
						Field::Mozjpeg => {
							if let Ok(app) = map.next_value() {
								let app: App = app;
								out.set_mozjpeg(app);
							}
						},
						Field::Oxipng => {
							if let Ok(app) = map.next_value() {
								let app: App = app;
								out.set_oxipng(app);
							}
						},
						Field::Pngout => {
							if let Ok(app) = map.next_value() {
								let app: App = app;
								out.set_pngout(app);
							}
						},
						Field::Zopflipng => {
							if let Ok(app) = map.next_value() {
								let app: App = app;
								out.set_zopflipng(app);
							}
						},
					}
				}

				Ok(out)
			}
		}

		deserializer.deserialize_struct("CoreSettings", FIELDS, CoreSettingsVisitor)
	}
}

impl CoreSettings {
	// Quick apps.
	core_settings_quick_apps!(jpegoptim);
	core_settings_quick_apps!(mozjpeg);
	core_settings_quick_apps!(oxipng);
	core_settings_quick_apps!(pngout);
	core_settings_quick_apps!(zopflipng);



	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// Load From File.
	pub fn load<P> (path: P) -> CoreSettings
	where P: AsRef<Path> {
		use std::fs::File;
		use std::io::prelude::*;

		if path.as_ref().is_file() {
			if let Ok(mut f) = File::open(&path) {
				let mut buffer = String::new();
				if true == f.read_to_string(&mut buffer).is_ok() {
					let out: CoreSettings = serde_yaml::from_str(&buffer).unwrap_or(CoreSettings::default());
					return out;
				}
			}
		}

		CoreSettings::default()
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Dry Run.
	pub fn dry_run(&self) -> bool {
		self.dry_run
	}

	/// Reporting Level.
	pub fn level(&self) -> usize {
		self.level
	}



	// -----------------------------------------------------------------
	// Setters
	// -----------------------------------------------------------------

	/// Dry Run.
	pub fn set_dry_run(&mut self, dry_run: bool) {
		self.dry_run = dry_run;
	}

	/// Reporting Level.
	pub fn set_level(&mut self, mut level: usize) {
		// The ceiling is four.
		if 4 < level {
			level = 4;
		}

		self.level = level;
	}
}


#[cfg(test)]
mod tests {
	use super::*;



	#[test]
	#[ignore]
	/// Test Reporter Operations.
	fn test_core_ops() {
		let core: Core = Core::new(CoreSettings::default());
		let state: Arc<Mutex<CoreState>> = core.state();

		// We don't want to make real changes.
		CoreState::arc_set_dry_run(state.clone(), true);
		assert!(CoreState::arc_dry_run(state.clone()));

		// Make sure reporting is set to 3.
		CoreState::arc_set_level(state.clone(), 3);
		assert_eq!(CoreState::arc_level(state.clone()), 3);

		let has_jpegs: bool = CoreState::arc_has_image_apps(state.clone(), ImageKind::Jpeg);
		let has_pngs: bool = CoreState::arc_has_image_apps(state.clone(), ImageKind::Png);

		// Test the internal app validation logic with the default paths
		// starting with JPEG apps.
		match CoreState::arc_image_apps(state.clone(), ImageKind::Jpeg) {
			Some(_) => assert!(has_jpegs),
			_ => assert_eq!(has_jpegs, false),
		}

		// And the same for PNG apps.
		match CoreState::arc_image_apps(state.clone(), ImageKind::Png) {
			Some(_) => assert!(has_pngs),
			_ => assert_eq!(has_pngs, false),
		}

		let paths: Vec<PathBuf> = vec![PathBuf::from("./tests/assets").flaca_to_abs_pathbuf()];
		assert!(core.run(&paths).is_ok());
	}
}
