/*!
# Images
*/

use crate::error::Error;
use crate::format;
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};
use std::ffi::{OsStr, OsString};
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};




#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// The kind of image.
pub enum ImageKind {
	/// JPEG.
	Jpeg,
	/// PNG.
	Png,
	/// Neither.
	None,
}

impl fmt::Display for ImageKind {
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match *self {
			Self::Jpeg => "JPEG",
			Self::Png => "PNG",
			Self::None => "",
		})
	}
}



#[derive(Debug, Clone, PartialEq, Eq, Hash)]
/// Image Applications.
///
/// Flaca is merely a convenient wrapper for separate, specialized image
/// optimization tools. The details for each application are handled
/// here.
pub enum App {
	/// Jpegoptim.
	Jpegoptim(PathBuf),
	/// MozJPEG.
	Mozjpeg(PathBuf),
	/// Oxipng.
	Oxipng(PathBuf),
	/// Pngout.
	Pngout(PathBuf),
	/// Zopflipng.
	Zopflipng(PathBuf),
	/// None.
	None,
}

impl fmt::Display for App {
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match *self {
			Self::Jpegoptim(_) => "Jpegoptim",
			Self::Mozjpeg(_) => "MozJPEG",
			Self::Oxipng(_) => "Oxipng",
			Self::Pngout(_) => "PNGOUT",
			Self::Zopflipng(_) => "Zopflipng",
			Self::None => "",
		})
	}
}

impl<T: Into<PathBuf>> From<T> for App {
	/// Derive App From Path.
	///
	/// This method attempts to convert an application path into a valid
	/// App Enum. Each application has its own rather specific name
	/// so this more or less works.
	///
	/// Nonetheless, it is recommended one use more specific assignment
	/// when possible to avoid weirdness.
	fn from(path: T) -> Self {
		let path: PathBuf = path.into();

		// It must be an executable file.
		if false == format::path::is_executable(&path) {
			return Self::None;
		}

		let name: String = format::path::file_name(&path)
			.to_str()
			.unwrap_or("")
			.to_string()
			.to_lowercase();

		if name.contains("jpegoptim") {
			Self::Jpegoptim(format::path::abs_pathbuf(&path))
		}
		else if name.contains("jpegtran") {
			Self::Mozjpeg(format::path::abs_pathbuf(&path))
		}
		else if name.contains("oxipng") {
			Self::Oxipng(format::path::abs_pathbuf(&path))
		}
		else if name.contains("pngout") {
			Self::Pngout(format::path::abs_pathbuf(&path))
		}
		else if name.contains("zopflipng") {
			Self::Zopflipng(format::path::abs_pathbuf(&path))
		}
		else {
			Self::None
		}
	}
}

impl Serialize for App {
	/// Serialize!
	fn serialize<S> (&self, serializer: S) -> Result<S::Ok, S::Error>
	where S: Serializer {
		let path: String = match &self.path() {
			Ok(ref p) => format::path::as_string(&p),
			_ => "".to_string(),
		};
		serializer.serialize_str(&path)
	}
}

impl<'de> Deserialize<'de> for App {
	/// Derialize!
	fn deserialize<D> (deserializer: D) -> Result<App, D::Error>
	where D: Deserializer<'de> {
		// Deserialize from a human-readable string like "2015-05-15T17:01:00Z".
		let s = String::deserialize(deserializer)?;
		Ok(App::from(&s))
	}
}

impl App {
	// Quick apps.
	quick_apps!("jpegoptim", Jpegoptim);
	quick_apps!("mozjpeg", Mozjpeg);
	quick_apps!("oxipng", Oxipng);
	quick_apps!("pngout", Pngout);
	quick_apps!("zopflipng", Zopflipng);



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Image Kind.
	///
	/// Return the ImageKind this App is capable of processing.
	pub fn image_kind(&self) -> ImageKind {
		if false == self.is_valid() {
			return ImageKind::None;
		}

		match *self {
			Self::Jpegoptim(_) => ImageKind::Jpeg,
			Self::Mozjpeg(_) => ImageKind::Jpeg,
			Self::Oxipng(_) => ImageKind::Png,
			Self::Pngout(_) => ImageKind::Png,
			Self::Zopflipng(_) => ImageKind::Png,
			Self::None => ImageKind::None,
		}
	}

	/// The App Path.
	///
	/// Return the PathBuf component of an App, provided the path
	/// appears to be a valid executable for this app.
	pub fn path(&self) -> Result<PathBuf, Error> {
		if false == self.is_valid() {
			return Err(Error::InvalidApp);
		}

		match *self {
			Self::Jpegoptim(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Mozjpeg(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Oxipng(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Pngout(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Zopflipng(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::None => Err(Error::InvalidApp),
		}
	}

	/// Slug.
	///
	/// Return the "normal" application file name for this App.
	pub fn slug(&self) -> OsString {
		OsStr::new(match *self {
			Self::Jpegoptim(_) => "jpegoptim",
			Self::Mozjpeg(_) => "jpegtran",
			Self::Oxipng(_) => "oxipng",
			Self::Pngout(_) => "pngout",
			Self::Zopflipng(_) => "zopflipng",
			_ => "",
		}).to_os_string()
	}



	// -----------------------------------------------------------------
	// Conversion
	// -----------------------------------------------------------------

	/// Cloned Copy
	///
	/// Clone the source if valid, otherwise return App::None.
	pub fn cloned(&self) -> App {
		match self.is_valid() {
			true => self.clone(),
			false => App::None,
		}
	}



	// -----------------------------------------------------------------
	// Evaluation
	// -----------------------------------------------------------------

	/// Is Valid?
	///
	/// This method attempts to answer whether or not a given App
	/// is present and executable on the system.
	///
	/// Unfortunately beyond checking those two things, we don't really
	/// have any good way of ensuring that the executable path is for
	/// the App it is supposed to be for.
	///
	/// Executing arbitrary programs is generally frowned upon, so as a
	/// least-terrible mitigation effort, the path names are checked
	/// against the "normal" application names. This matching is done
	/// in case-insensitive needle/haystack fashion, so the name does
	/// not have to match _exactly_; it merely has to contain the
	/// `::slug()` somewhere in its name.
	pub fn is_valid(&self) -> bool {
		let path: PathBuf = match *self {
			Self::Jpegoptim(ref p) => format::path::abs_pathbuf(&p),
			Self::Mozjpeg(ref p) => format::path::abs_pathbuf(&p),
			Self::Oxipng(ref p) => format::path::abs_pathbuf(&p),
			Self::Pngout(ref p) => format::path::abs_pathbuf(&p),
			Self::Zopflipng(ref p) => format::path::abs_pathbuf(&p),
			Self::None => return false,
		};

		// It must be an executable file.
		if false == format::path::is_executable(&path) {
			return false;
		}

		// Make sure the file name looks like the slug.
		let name: String = format::path::file_name(&path)
			.to_str()
			.unwrap_or("")
			.to_string()
			.to_lowercase();

		name.contains(self.slug().to_str().unwrap_or(""))
	}



	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Compress an Image!
	///
	/// This method will run the App binary to losslessly compress
	/// the source image as much as possible.
	///
	/// The source image will be overwritten at the end of the process
	/// unless no savings were found.
	pub fn compress<P> (&self, path: P) -> Result<(), Error>
	where P: AsRef<Path> {
		// This will also test validity.
		let bin_path: PathBuf = self.path()?;

		// Make sure the image type is right for the app.
		if false == format::path::is_image_kind(&path, self.image_kind()) {
			return match self.image_kind() {
				ImageKind::Jpeg => Err(Error::NotJpeg(format::path::as_string(&path))),
				ImageKind::Png => Err(Error::NotPng(format::path::as_string(&path))),
				_ => Err(Error::InvalidPath(format::path::as_string(&path))),
			};
		}

		// Our starting size.
		let start_size: usize = format::path::file_size(&path);

		// We need to make a working copy.
		let working = format::path::tmp_copy_file(&path)?;

		// Some programs want to write changes to a third location, so
		// let's give them somewhere to do it.
		let mut working2 = working.as_os_str().to_os_string();
		working2.push(".bak");
		let working2: PathBuf = format::path::as_unique_pathbuf(&working2)?;

		// Build a command.
		let mut com = Command::new(bin_path);
		match *self {
			Self::Jpegoptim(_) => {
				com.arg("-q");
				com.arg("-f");
				com.arg("--strip-all");
				com.arg("--all-progressive");
				com.arg(&working);
			},
			Self::Mozjpeg(_) => {
				com.arg("-copy");
				com.arg("none");
				com.arg("-optimize");
				com.arg("-progressive");
				com.arg("-outfile");
				com.arg(&working2);
				com.arg(&working);
			},
			Self::Oxipng(_) => {
				com.arg("-s");
				com.arg("-q");
				com.arg("--fix");
				com.arg("-o");
				com.arg("6");
				com.arg("-i");
				com.arg("0");
				com.arg(&working);
			},
			Self::Pngout(_) => {
				com.arg(&working);
				com.arg("-q");
			},
			Self::Zopflipng(_) => {
				com.arg("-m");
				com.arg(&working);
				com.arg(&working2);
			},
			Self::None => return Err(Error::NoApp(format!("{}", &self))),
		}

		// Run the command!
		com
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()?;

		// Replace the first copy with the second copy, if applicable.
		if working2.is_file() {
			format::path::move_file(&working2, &working)?;
		}

		// How'd we do?
		let end_size: usize = format::path::file_size(&working);
		if end_size == 0 {
			if working.is_file() {
				format::path::delete_file(&working)?;
			}
			return Err(Error::new("Image optimizer failed."));
		}

		// If we have a smaller file, replace it.
		if end_size < start_size {
			format::path::move_file_bytes(&working, &path)?;
		}
		// Clean up is needed.
		else if working.is_file() {
			format::path::delete_file(&working)?;
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// Test App::None.
	fn test_image_app_none() {
		// Test non-app wrappers.
		assert_eq!(App::None.is_valid(), false);
		assert!(App::None.path().is_err());
		assert_eq!(App::None.image_kind(), ImageKind::None);
	}

	#[test]
	/// Test App::Jpegoptim.
	fn test_image_app_jpegoptim() {
		// We have to find it first.
		match App::find_jpegoptim() {
			App::Jpegoptim(ref p) => {
				let app: App = App::try_jpegoptim(p.to_path_buf());
				assert_eq!(
					app,
					App::Jpegoptim(p.to_path_buf())
				);
				assert_eq!(
					app,
					App::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Jpeg);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize App.");
				let unyaml: App = serde_yaml::from_str(&yaml).expect("Unable to deserialize App.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			App::try_jpegoptim(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			App::None,
		);
	}

	#[test]
	/// Test App::Mozjpeg.
	fn test_image_app_mozjpeg() {
		// We have to find it first.
		match App::find_mozjpeg() {
			App::Mozjpeg(ref p) => {
				let app: App = App::try_mozjpeg(p.to_path_buf());
				assert_eq!(
					app,
					App::Mozjpeg(p.to_path_buf())
				);
				assert_eq!(
					app,
					App::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Jpeg);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize App.");
				let unyaml: App = serde_yaml::from_str(&yaml).expect("Unable to deserialize App.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			App::try_mozjpeg(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			App::None,
		);
	}

	#[test]
	/// Test App::Oxipng.
	fn test_image_app_oxipng() {
		// We have to find it first.
		match App::find_oxipng() {
			App::Oxipng(ref p) => {
				let app: App = App::try_oxipng(p.to_path_buf());
				assert_eq!(
					app,
					App::Oxipng(p.to_path_buf())
				);
				assert_eq!(
					app,
					App::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize App.");
				let unyaml: App = serde_yaml::from_str(&yaml).expect("Unable to deserialize App.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			App::try_oxipng(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			App::None,
		);
	}

	#[test]
	/// Test App::Pngout.
	fn test_image_app_pngout() {
		// We have to find it first.
		match App::find_pngout() {
			App::Pngout(ref p) => {
				let app: App = App::try_pngout(p.to_path_buf());
				assert_eq!(
					app,
					App::Pngout(p.to_path_buf())
				);
				assert_eq!(
					app,
					App::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize App.");
				let unyaml: App = serde_yaml::from_str(&yaml).expect("Unable to deserialize App.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			App::try_pngout(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			App::None,
		);
	}

	#[test]
	/// Test App::Zopflipng.
	fn test_image_app_zopflipng() {
		// We have to find it first.
		match App::find_zopflipng() {
			App::Zopflipng(ref p) => {
				let app: App = App::try_zopflipng(p.to_path_buf());
				assert_eq!(
					app,
					App::Zopflipng(p.to_path_buf())
				);
				assert_eq!(
					app,
					App::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize App.");
				let unyaml: App = serde_yaml::from_str(&yaml).expect("Unable to deserialize App.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			App::try_zopflipng(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			App::None,
		);
	}

	#[test]
	#[ignore]
	/// Test App JPEG Compression.
	fn test_image_app_jpeg_compression() {
		let jpg = format::path::abs_pathbuf("./tests/assets/01.jpg");

		// Test whichever apps are available.
		for (app, slug) in vec![
			(App::find_jpegoptim(), "jpegoptim"),
			(App::find_mozjpeg(), "jpegtran"),
		].iter() {
			// If the app isn't on the system we can't test its compression.
			if *app == App::None {
				continue;
			}

			// Make sure we pulled the right kind of app.
			assert_eq!(app.slug(), *slug);

			// Make a copy of the image for testing purposes.
			let image: PathBuf = format::path::tmp_copy_file(&jpg)
				.expect("Could not copy image file.");
			assert!(format::path::is_image_kind(&image, ImageKind::Jpeg));

			// Grab its size too for later comparison.
			let before: usize = format::path::file_size(&image);
			assert!(0 < before);

			// Compress it!
			assert!(app.compress(&image).is_ok());

			// Make sure the image is still valid. Should be, but you
			// never know!
			assert!(format::path::is_image_kind(&image, ImageKind::Jpeg));

			// Check the size again.
			let after: usize = format::path::file_size(&image);
			assert!(0 < after);

			// This should be smaller now.
			assert!(after < before);

			// And clean up after ourselves.
			if image.exists() {
				assert!(format::path::delete_file(&image).is_ok());
			}
		}
	}

	#[test]
	#[ignore]
	/// Test App PNG Compression.
	fn test_image_app_png_compression() {
		let png = format::path::abs_pathbuf("./tests/assets/02.png");

		// Test whichever apps are available.
		for (app, slug) in vec![
			(App::find_oxipng(), "oxipng"),
			(App::find_pngout(), "pngout"),
			(App::find_zopflipng(), "zopflipng"),
		].iter() {
			// If the app isn't on the system we can't test its compression.
			if *app == App::None {
				continue;
			}

			// Make sure we pulled the right kind of app.
			assert_eq!(app.slug(), *slug);

			// Make a copy of the image for testing purposes.
			let image: PathBuf = format::path::tmp_copy_file(&png)
				.expect("Could not copy image file.");
			assert!(format::path::is_image_kind(&image, ImageKind::Png));

			// Grab its size too for later comparison.
			let before: usize = format::path::file_size(&image);
			assert!(0 < before);

			// Compress it!
			assert!(app.compress(&image).is_ok());

			// Make sure the image is still valid. Should be, but you
			// never know!
			assert!(format::path::is_image_kind(&image, ImageKind::Png));

			// Check the size again.
			let after: usize = format::path::file_size(&image);
			assert!(0 < after);

			// This should be smaller now.
			assert!(after < before);

			// And clean up after ourselves.
			if image.exists() {
				assert!(format::path::delete_file(&image).is_ok());
			}
		}
	}
}
