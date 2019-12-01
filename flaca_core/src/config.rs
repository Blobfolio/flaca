/*!
# Configuration
*/

use crate::image::{ImageApp, ImageKind};
use crate::reporter::Reporter;
use serde::de::{Deserialize, Deserializer, Visitor, MapAccess};
use serde::ser::{Serialize, Serializer, SerializeStruct};
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};



#[derive(Debug, Clone)]
/// Configuration.
pub struct Config {
	/// Dry Run.
	dry_run: Arc<AtomicBool>,
	/// Reporting Level.
	level: Arc<AtomicUsize>,
	/// Reporter.
	reporter: Arc<Mutex<Reporter>>,
	/// Jpegoptim.
	jpegoptim: Arc<Mutex<ImageApp>>,
	/// MozJPEG.
	mozjpeg: Arc<Mutex<ImageApp>>,
	/// Oxipng.
	oxipng: Arc<Mutex<ImageApp>>,
	/// PNGOUT.
	pngout: Arc<Mutex<ImageApp>>,
	/// Zopflipng.
	zopflipng: Arc<Mutex<ImageApp>>,
}

impl Default for Config {
	/// Default.
	fn default() -> Config {
		Config {
			dry_run: Arc::new(AtomicBool::new(false)),
			level: Arc::new(AtomicUsize::new(3)),
			reporter: Arc::new(Mutex::new(Reporter::default())),
			jpegoptim: Arc::new(Mutex::new(ImageApp::find_jpegoptim())),
			mozjpeg: Arc::new(Mutex::new(ImageApp::find_mozjpeg())),
			oxipng: Arc::new(Mutex::new(ImageApp::find_oxipng())),
			pngout: Arc::new(Mutex::new(ImageApp::find_pngout())),
			zopflipng: Arc::new(Mutex::new(ImageApp::find_zopflipng())),
		}
	}
}

impl Serialize for Config {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		// 3 is the number of fields in the struct.
		let mut state = serializer.serialize_struct("FlacaConfig", 6)?;
		state.serialize_field("level", &self.level())?;
		state.serialize_field("jpegoptim", &self.jpegoptim())?;
		state.serialize_field("mozjpeg", &self.mozjpeg())?;
		state.serialize_field("oxipng", &self.oxipng())?;
		state.serialize_field("pngout", &self.pngout())?;
		state.serialize_field("zopflipng", &self.zopflipng())?;
		state.end()
	}
}

impl<'de> Deserialize<'de> for Config {
	/// Derialize!
	fn deserialize<D> (deserializer: D) -> Result<Config, D::Error>
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
		struct ConfigVisitor;
		impl<'de> Visitor<'de> for ConfigVisitor {
			type Value = Config;

			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				formatter.write_str("struct Config")
			}

			fn visit_map<V>(self, mut map: V) -> Result<Config, V::Error>
			where
				V: MapAccess<'de>,
			{
				let out: Config = Config::default();

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
								let app: ImageApp = app;
								out.set_jpegoptim(app);
							}
						},
						Field::Mozjpeg => {
							if let Ok(app) = map.next_value() {
								let app: ImageApp = app;
								out.set_mozjpeg(app);
							}
						},
						Field::Oxipng => {
							if let Ok(app) = map.next_value() {
								let app: ImageApp = app;
								out.set_oxipng(app);
							}
						},
						Field::Pngout => {
							if let Ok(app) = map.next_value() {
								let app: ImageApp = app;
								out.set_pngout(app);
							}
						},
						Field::Zopflipng => {
							if let Ok(app) = map.next_value() {
								let app: ImageApp = app;
								out.set_zopflipng(app);
							}
						},
					}
				}

				Ok(out)
			}
		}

		deserializer.deserialize_struct("FlacaConfig", FIELDS, ConfigVisitor)
	}
}

impl Config {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// Load From File.
	pub fn load<P> (path: P) -> Config
	where P: AsRef<Path> {
		if path.as_ref().is_file() {
			if let Ok(mut f) = File::open(&path) {
				let mut buffer = String::new();
				if true == f.read_to_string(&mut buffer).is_ok() {
					let out: Config = serde_yaml::from_str(&buffer).unwrap_or(Config::default());
					return out;
				}
			}
		}

		Config::default()
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Dry Run.
	pub fn dry_run(&self) -> bool {
		let ptr = self.dry_run.clone();
		ptr.load(Ordering::Relaxed)
	}

	/// List of Available Image Apps (By Name).
	pub fn image_app_list(&self) -> Vec<String> {
		let mut out: Vec<String> = Vec::new();

		if ImageApp::None != self.jpegoptim() {
			out.push("Jpegoptim".to_string());
		}
		if ImageApp::None != self.mozjpeg() {
			out.push("MozJPEG".to_string());
		}
		if ImageApp::None != self.oxipng() {
			out.push("Oxipng".to_string());
		}
		if ImageApp::None != self.pngout() {
			out.push("Pngout".to_string());
		}
		if ImageApp::None != self.zopflipng() {
			out.push("Zopflipng".to_string());
		}

		out
	}

	/// Available Image Apps.
	pub fn image_apps(&self, kind: ImageKind) -> Option<Vec<ImageApp>> {
		let mut out: Vec<ImageApp> = Vec::new();

		match kind {
			ImageKind::Jpeg => {
				let app = self.jpegoptim();
				if ImageApp::None != app {
					out.push(app);
				}

				let app = self.mozjpeg();
				if ImageApp::None != app {
					out.push(app);
				}
			},
			ImageKind::Png => {
				let app = self.pngout();
				if ImageApp::None != app {
					out.push(app);
				}

				let app = self.oxipng();
				if ImageApp::None != app {
					out.push(app);
				}

				let app = self.zopflipng();
				if ImageApp::None != app {
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

	/// Reporting Level.
	pub fn level(&self) -> usize {
		let ptr = self.level.clone();
		ptr.load(Ordering::Relaxed)
	}

	/// Jpegoptim.
	pub fn jpegoptim(&self) -> ImageApp {
		let ptr = self.jpegoptim.clone();
		let a = ptr.lock().unwrap();
		match a.is_valid() {
			true => a.clone(),
			false => ImageApp::None,
		}
	}

	/// MozJPEG.
	pub fn mozjpeg(&self) -> ImageApp {
		let ptr = self.mozjpeg.clone();
		let a = ptr.lock().unwrap();
		match a.is_valid() {
			true => a.clone(),
			false => ImageApp::None,
		}
	}

	/// Oxipng.
	pub fn oxipng(&self) -> ImageApp {
		let ptr = self.oxipng.clone();
		let a = ptr.lock().unwrap();
		match a.is_valid() {
			true => a.clone(),
			false => ImageApp::None,
		}
	}

	/// Pngout.
	pub fn pngout(&self) -> ImageApp {
		let ptr = self.pngout.clone();
		let a = ptr.lock().unwrap();
		match a.is_valid() {
			true => a.clone(),
			false => ImageApp::None,
		}
	}

	/// Reporter.
	pub fn reporter(&self) -> Arc<Mutex<Reporter>> {
		self.reporter.clone()
	}

	/// Zopflipng.
	pub fn zopflipng(&self) -> ImageApp {
		let ptr = self.zopflipng.clone();
		let a = ptr.lock().unwrap();
		match a.is_valid() {
			true => a.clone(),
			false => ImageApp::None,
		}
	}



	// -----------------------------------------------------------------
	// Setters
	// -----------------------------------------------------------------

	/// Dry Run.
	pub fn set_dry_run(&self, dry_run: bool) {
		let ptr = self.dry_run.clone();
		ptr.store(dry_run, Ordering::Relaxed);

		Reporter::arc_set_dry_run(self.reporter.clone(), dry_run);
	}

	/// Reporting Level.
	pub fn set_level(&self, mut level: usize) {
		let ptr = self.level.clone();

		// The ceiling is four.
		if 4 < level {
			level = 4;
		}

		ptr.store(level, Ordering::Relaxed);
		Reporter::arc_set_level(self.reporter.clone(), level);
	}

	/// Jpegoptim.
	pub fn set_jpegoptim(&self, app: ImageApp) {
		let ptr = self.jpegoptim.clone();
		let mut a = ptr.lock().unwrap();
		*a = match app.is_valid() {
			true => app,
			false => ImageApp::None,
		};
	}

	/// MozJPEG.
	pub fn set_mozjpeg(&self, app: ImageApp) {
		let ptr = self.mozjpeg.clone();
		let mut a = ptr.lock().unwrap();
		*a = match app.is_valid() {
			true => app,
			false => ImageApp::None,
		};
	}

	/// Oxipng.
	pub fn set_oxipng(&self, app: ImageApp) {
		let ptr = self.oxipng.clone();
		let mut a = ptr.lock().unwrap();
		*a = match app.is_valid() {
			true => app,
			false => ImageApp::None,
		};
	}

	/// Pngout.
	pub fn set_pngout(&self, app: ImageApp) {
		let ptr = self.pngout.clone();
		let mut a = ptr.lock().unwrap();
		*a = match app.is_valid() {
			true => app,
			false => ImageApp::None,
		};
	}

	/// Zopflipng.
	pub fn set_zopflipng(&self, app: ImageApp) {
		let ptr = self.zopflipng.clone();
		let mut a = ptr.lock().unwrap();
		*a = match app.is_valid() {
			true => app,
			false => ImageApp::None,
		};
	}



	// -----------------------------------------------------------------
	// Evaluation
	// -----------------------------------------------------------------

	/// Has Apps?
	///
	/// Return whether or not at least one ImageApp exists for the image
	/// type.
	pub fn has_image_apps(&self, kind: ImageKind) -> bool {
		match kind {
			ImageKind::Jpeg =>
				ImageApp::None != self.jpegoptim() ||
				ImageApp::None != self.mozjpeg(),
			ImageKind::Png =>
				ImageApp::None != self.oxipng() ||
				ImageApp::None != self.pngout() ||
				ImageApp::None != self.zopflipng(),
			_ => false,
		}
	}

	/// Is Running?
	pub fn is_running(&self) -> bool {
		Reporter::arc_running(self.reporter.clone())
	}



	// -----------------------------------------------------------------
	// Arc Wrappers
	// -----------------------------------------------------------------

	/// Dry Run.
	pub fn arc_dry_run(config: Arc<Mutex<Config>>) -> bool {
		let c = config.lock().unwrap();
		c.dry_run()
	}

	/// Image App List (By Name).
	pub fn arc_image_app_list(config: Arc<Mutex<Config>>) -> Vec<String> {
		let c = config.lock().unwrap();
		c.image_app_list()
	}

	/// Image Apps.
	pub fn arc_image_apps(config: Arc<Mutex<Config>>, kind: ImageKind) -> Option<Vec<ImageApp>> {
		let c = config.lock().unwrap();
		c.image_apps(kind)
	}

	/// Level.
	pub fn arc_level(config: Arc<Mutex<Config>>) -> usize {
		let c = config.lock().unwrap();
		c.level()
	}

	/// Reporter.
	pub fn arc_reporter(config: Arc<Mutex<Config>>) -> Arc<Mutex<Reporter>> {
		let c = config.lock().unwrap();
		c.reporter()
	}

	/// Has Image Apps?
	pub fn arc_has_image_apps(config: Arc<Mutex<Config>>, kind: ImageKind) -> bool {
		let c = config.lock().unwrap();
		c.has_image_apps(kind)
	}

	/// Is Running?
	pub fn arc_is_running(config: Arc<Mutex<Config>>) -> bool {
		let c = config.lock().unwrap();
		c.is_running()
	}
}
