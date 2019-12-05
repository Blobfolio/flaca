/*!
# Macros
*/

/// Atomic Load/Store.
///
/// This macro provides shorthand methods for loading and storing
/// AtomicBool, AtomicUsize, etc., types wrapped inside Arc pointers.
macro_rules! atomicity {
	($arc:expr) => {
		{
			let ptr = $arc.clone();
			ptr.load(Ordering::SeqCst)
		}
	};

	($arc:expr, $val:expr) => {
		{
			let ptr = $arc.clone();
			ptr.store($val, Ordering::SeqCst)
		}
	};
}

/// Generate Apps.
///
/// There is a lot of redundancy among the various app-specific methods.
/// This macro helps tidy the code a bit.
macro_rules! quick_apps {
	($name:expr, $key:ident) => {
		paste::item! {
			/// Find $key.
			///
			/// Look for the image app in its default place. If found,
			/// an ImageApp::$key will be returned; if not, an
			/// ImageApp::None will be returned.
			///
			/// Executables can, of course, live anywhere and be called
			/// anything, so this should only serve as a sane fallback.
			pub fn [<find_ $name>]() -> Self {
				// Mozjpeg requires special handling.
				match "mozjpeg" == $name {
					true => {
						let p: PathBuf = PathBuf::from("/opt/mozjpeg/bin/jpegtran");
						match format::path::is_executable(&p) {
							true => Self::Mozjpeg(p),
							false => Self::None,
						}
					},
					false => match format::path::find_executable($name) {
						Some(p) => Self::$key(p),
						_ => Self::None,
					}
				}
			}

			/// Try $key.
			///
			/// This will return an App::$key instance if the path
			/// is valid, otherwise an App::None.
			pub fn [<try_ $name>]<P> (path: P) -> Self
			where P: AsRef<Path> {
				let out: Self = Self::$key(format::path::abs_pathbuf(path));
				match out.is_valid() {
					true => out,
					false => Self::None,
				}
			}

			/// Is $key?
			pub fn [<is_ $name>](&self) -> bool {
				match self {
					Self::$key(_) => self.is_valid(),
					_ => false,
				}
			}
		}
	};
}

/// Generate CoreSettings Apps.
///
/// The app methods for CoreSettings are very redundant; this macro
/// helps tidy the code a bit.
macro_rules! core_settings_quick_apps {
	($name:ident) => {
		paste::item! {
			/// $name.
			pub fn $name(&self) -> App {
				self.$name.cloned()
			}

			/// Set $name.
			pub fn [<set_ $name>] (&mut self, app: App) {
				self.$name = match app.[<is_ $name>]() {
					true => app,
					false => App::None,
				}
			}
		}
	};
}

/// Generate CoreState Apps.
///
/// The app methods for CoreState are very redundant; this macro helps
/// tidy the code a bit.
macro_rules! core_state_quick_apps {
	($name:ident) => {
		paste::item! {
			/// $name.
			pub fn $name(&self) -> App {
				let ptr = self.$name.clone();
				let a = ptr.lock().unwrap();
				a.cloned()
			}

			/// Set $name.
			pub fn [<set_ $name>] (&self, app: App) {
				let ptr = self.$name.clone();
				let mut a = ptr.lock().unwrap();
				*a = match app.[<is_ $name>]() {
					true => app,
					false => App::None,
				};
			}

			/// $name (Arc).
			pub fn [<arc_ $name>](state: Arc<Mutex<CoreState>>) -> App {
				let c = state.lock().unwrap();
				c.$name()
			}

			/// Set $name (Arc).
			pub fn [<arc_set_ $name>](state: Arc<Mutex<CoreState>>, app: App) {
				let c = state.lock().unwrap();
				c.[<set_ $name>](app)
			}
		}
	};
}
