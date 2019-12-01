/*!
# Images
*/

use crate::error::FlacaError;
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
	#[inline]
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
pub enum ImageApp {
	/// Jpegoptim.
	Jpegoptim(PathBuf),
	/// MozJPEG.
	MozJPEG(PathBuf),
	/// Oxipng.
	Oxipng(PathBuf),
	/// Pngout.
	Pngout(PathBuf),
	/// Zopflipng.
	Zopflipng(PathBuf),
	/// None.
	None,
}

impl fmt::Display for ImageApp {
	#[inline]
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", match *self {
			Self::Jpegoptim(_) => "Jpegoptim",
			Self::MozJPEG(_) => "MozJPEG",
			Self::Oxipng(_) => "Oxipng",
			Self::Pngout(_) => "PNGOUT",
			Self::Zopflipng(_) => "Zopflipng",
			Self::None => "",
		})
	}
}

impl<T: Into<PathBuf>> From<T> for ImageApp {
	/// Derive ImageApp From Path.
	///
	/// This method attempts to convert an application path into a valid
	/// ImageApp Enum. Each application has its own rather specific name
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
			Self::MozJPEG(format::path::abs_pathbuf(&path))
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

impl Serialize for ImageApp {
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

impl<'de> Deserialize<'de> for ImageApp {
	/// Derialize!
	fn deserialize<D> (deserializer: D) -> Result<ImageApp, D::Error>
	where D: Deserializer<'de> {
		// Deserialize from a human-readable string like "2015-05-15T17:01:00Z".
		let s = String::deserialize(deserializer)?;
		Ok(ImageApp::from(&s))
	}
}

impl ImageApp {
	// -----------------------------------------------------------------
	// Construction
	// -----------------------------------------------------------------

	/// Find Jpegoptim.
	///
	/// This will look for Jpegoptim under the user's $PATH. If found,
	/// an ImageApp::Jpegoptim is returned, otherwise an ImageApp::None.
	///
	/// Executables can, of course, live anywhere and be called
	/// anything, so this should only serve as a sane fallback.
	pub fn find_jpegoptim() -> Self {
		match format::path::find_executable("jpegoptim") {
			Some(p) => Self::Jpegoptim(p),
			_ => Self::None,
		}
	}

	/// Find MozJPEG.
	///
	/// Unlike the other applications, MozJPEG's executable shares a
	/// name with a more prominent (albeit less useful) library.
	///
	/// Rather than looking under all of the executable $PATH for the
	/// user, this will look in the default place the MozJPEG installer
	/// uses. If nothing executable lives there, an ImageApp::None is
	/// returned.
	///
	/// Executables can, of course, live anywhere and be called
	/// anything, so this should only serve as a sane fallback.
	pub fn find_mozjpeg() -> Self {
		let p: PathBuf = PathBuf::from("/opt/mozjpeg/bin/jpegtran");
		match format::path::is_executable(&p) {
			true => Self::MozJPEG(p),
			false => Self::None,
		}
	}

	/// Find Oxipng.
	///
	/// This will look for Oxipng under the user's $PATH. If found,
	/// an ImageApp::Oxipng is returned, otherwise an ImageApp::None.
	///
	/// Executables can, of course, live anywhere and be called
	/// anything, so this should only serve as a sane fallback.
	pub fn find_oxipng() -> Self {
		match format::path::find_executable("oxipng") {
			Some(p) => Self::Oxipng(p),
			_ => Self::None,
		}
	}

	/// Find Pngout.
	///
	/// This will look for Pngout under the user's $PATH. If found,
	/// an ImageApp::Pngout is returned, otherwise an ImageApp::None.
	///
	/// Executables can, of course, live anywhere and be called
	/// anything, so this should only serve as a sane fallback.
	pub fn find_pngout() -> Self {
		match format::path::find_executable("pngout") {
			Some(p) => Self::Pngout(p),
			_ => Self::None,
		}
	}

	/// Find Zopflipng.
	///
	/// This will look for Zopflipng under the user's $PATH. If found,
	/// an ImageApp::Zopflipng is returned, otherwise an ImageApp::None.
	///
	/// Executables can, of course, live anywhere and be called
	/// anything, so this should only serve as a sane fallback.
	pub fn find_zopflipng() -> Self {
		match format::path::find_executable("zopflipng") {
			Some(p) => Self::Zopflipng(p),
			_ => Self::None,
		}
	}

	/// Try Jpegoptim.
	///
	/// This will return an ImageApp::Jpegoptim instance if the path
	/// is valid, otherwise an ImageApp::None.
	pub fn try_jpegoptim<P> (path: P) -> Self
	where P: AsRef<Path> {
		let out: Self = Self::Jpegoptim(format::path::abs_pathbuf(path));
		match out.is_valid() {
			true => out,
			false => Self::None,
		}
	}

	/// Try MozJPEG.
	///
	/// This will return an ImageApp::MozJPEG instance if the path
	/// is valid, otherwise an ImageApp::None.
	pub fn try_mozjpeg<P> (path: P) -> Self
	where P: AsRef<Path> {
		let out: Self = Self::MozJPEG(format::path::abs_pathbuf(path));
		match out.is_valid() {
			true => out,
			false => Self::None,
		}
	}

	/// Try Oxipng.
	///
	/// This will return an ImageApp::Oxipng instance if the path
	/// is valid, otherwise an ImageApp::None.
	pub fn try_oxipng<P> (path: P) -> Self
	where P: AsRef<Path> {
		let out: Self = Self::Oxipng(format::path::abs_pathbuf(path));
		match out.is_valid() {
			true => out,
			false => Self::None,
		}
	}

	/// Try Pngout.
	///
	/// This will return an ImageApp::Pngout instance if the path
	/// is valid, otherwise an ImageApp::None.
	pub fn try_pngout<P> (path: P) -> Self
	where P: AsRef<Path> {
		let out: Self = Self::Pngout(format::path::abs_pathbuf(path));
		match out.is_valid() {
			true => out,
			false => Self::None,
		}
	}

	/// Try Zopflipng.
	///
	/// This will return an ImageApp::Zopflipng instance if the path
	/// is valid, otherwise an ImageApp::None.
	pub fn try_zopflipng<P> (path: P) -> Self
	where P: AsRef<Path> {
		let out: Self = Self::Zopflipng(format::path::abs_pathbuf(path));
		match out.is_valid() {
			true => out,
			false => Self::None,
		}
	}



	// -----------------------------------------------------------------
	// Getters
	// -----------------------------------------------------------------

	/// Image Kind.
	///
	/// Return the ImageKind this ImageApp is capable of processing.
	pub fn image_kind(&self) -> ImageKind {
		if false == self.is_valid() {
			return ImageKind::None;
		}

		match *self {
			Self::Jpegoptim(_) => ImageKind::Jpeg,
			Self::MozJPEG(_) => ImageKind::Jpeg,
			Self::Oxipng(_) => ImageKind::Png,
			Self::Pngout(_) => ImageKind::Png,
			Self::Zopflipng(_) => ImageKind::Png,
			Self::None => ImageKind::None,
		}
	}

	/// The App Path.
	///
	/// Return the PathBuf component of an ImageApp, provided the path
	/// appears to be a valid executable for this app.
	pub fn path(&self) -> Result<PathBuf, FlacaError> {
		if false == self.is_valid() {
			return Err(FlacaError::MissingWorker);
		}

		match *self {
			Self::Jpegoptim(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::MozJPEG(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Oxipng(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Pngout(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::Zopflipng(ref p) => Ok(format::path::abs_pathbuf(&p)),
			Self::None => Err(FlacaError::MissingWorker),
		}
	}

	/// Slug.
	///
	/// Return the "normal" application file name for this ImageApp.
	pub fn slug(&self) -> OsString {
		OsStr::new(match *self {
			Self::Jpegoptim(_) => "jpegoptim",
			Self::MozJPEG(_) => "jpegtran",
			Self::Oxipng(_) => "oxipng",
			Self::Pngout(_) => "pngout",
			Self::Zopflipng(_) => "zopflipng",
			_ => "",
		}).to_os_string()
	}



	// -----------------------------------------------------------------
	// Evaluation
	// -----------------------------------------------------------------

	/// Is Valid?
	///
	/// This method attempts to answer whether or not a given ImageApp
	/// is present and executable on the system.
	///
	/// Unfortunately beyond checking those two things, we don't really
	/// have any good way of ensuring that the executable path is for
	/// the ImageApp it is supposed to be for.
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
			Self::MozJPEG(ref p) => format::path::abs_pathbuf(&p),
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
	/// This method will run the ImageApp binary to losslessly compress
	/// the source image as much as possible.
	///
	/// The source image will be overwritten at the end of the process
	/// unless no savings were found.
	pub fn compress<P> (&self, path: P) -> Result<(), FlacaError>
	where P: AsRef<Path> {
		// This will also test validity.
		let bin_path: PathBuf = self.path()?;

		// Make sure the image type is right for the app.
		if false == format::path::is_image_kind(&path, self.image_kind()) {
			return match self.image_kind() {
				ImageKind::Jpeg => Err(FlacaError::NotJpeg),
				ImageKind::Png => Err(FlacaError::NotPng),
				_ => Err(FlacaError::NotImage),
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
			Self::MozJPEG(_) => {
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
			Self::None => return Err(FlacaError::MissingWorker),
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
			return Err(FlacaError::new("Image optimizer failed."));
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
	/// Test ImageApp::None.
	fn test_image_app_none() {
		// Test non-app wrappers.
		assert_eq!(ImageApp::None.is_valid(), false);
		assert!(ImageApp::None.path().is_err());
		assert_eq!(ImageApp::None.image_kind(), ImageKind::None);
	}

	#[test]
	/// Test ImageApp::Jpegoptim.
	fn test_image_app_jpegoptim() {
		// We have to find it first.
		match ImageApp::find_jpegoptim() {
			ImageApp::Jpegoptim(ref p) => {
				let app: ImageApp = ImageApp::try_jpegoptim(p.to_path_buf());
				assert_eq!(
					app,
					ImageApp::Jpegoptim(p.to_path_buf())
				);
				assert_eq!(
					app,
					ImageApp::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Jpeg);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize ImageApp.");
				let unyaml: ImageApp = serde_yaml::from_str(&yaml).expect("Unable to deserialize ImageApp.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			ImageApp::try_jpegoptim(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			ImageApp::None,
		);
	}

	#[test]
	/// Test ImageApp::MozJPEG.
	fn test_image_app_mozjpeg() {
		// We have to find it first.
		match ImageApp::find_mozjpeg() {
			ImageApp::MozJPEG(ref p) => {
				let app: ImageApp = ImageApp::try_mozjpeg(p.to_path_buf());
				assert_eq!(
					app,
					ImageApp::MozJPEG(p.to_path_buf())
				);
				assert_eq!(
					app,
					ImageApp::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Jpeg);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize ImageApp.");
				let unyaml: ImageApp = serde_yaml::from_str(&yaml).expect("Unable to deserialize ImageApp.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			ImageApp::try_mozjpeg(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			ImageApp::None,
		);
	}

	#[test]
	/// Test ImageApp::Oxipng.
	fn test_image_app_oxipng() {
		// We have to find it first.
		match ImageApp::find_oxipng() {
			ImageApp::Oxipng(ref p) => {
				let app: ImageApp = ImageApp::try_oxipng(p.to_path_buf());
				assert_eq!(
					app,
					ImageApp::Oxipng(p.to_path_buf())
				);
				assert_eq!(
					app,
					ImageApp::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize ImageApp.");
				let unyaml: ImageApp = serde_yaml::from_str(&yaml).expect("Unable to deserialize ImageApp.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			ImageApp::try_oxipng(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			ImageApp::None,
		);
	}

	#[test]
	/// Test ImageApp::Pngout.
	fn test_image_app_pngout() {
		// We have to find it first.
		match ImageApp::find_pngout() {
			ImageApp::Pngout(ref p) => {
				let app: ImageApp = ImageApp::try_pngout(p.to_path_buf());
				assert_eq!(
					app,
					ImageApp::Pngout(p.to_path_buf())
				);
				assert_eq!(
					app,
					ImageApp::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize ImageApp.");
				let unyaml: ImageApp = serde_yaml::from_str(&yaml).expect("Unable to deserialize ImageApp.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			ImageApp::try_pngout(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			ImageApp::None,
		);
	}

	#[test]
	/// Test ImageApp::Zopflipng.
	fn test_image_app_zopflipng() {
		// We have to find it first.
		match ImageApp::find_zopflipng() {
			ImageApp::Zopflipng(ref p) => {
				let app: ImageApp = ImageApp::try_zopflipng(p.to_path_buf());
				assert_eq!(
					app,
					ImageApp::Zopflipng(p.to_path_buf())
				);
				assert_eq!(
					app,
					ImageApp::from(p.to_path_buf())
				);
				assert!(app.is_valid());
				assert!(app.path().is_ok());
				assert_eq!(app.image_kind(), ImageKind::Png);

				// Test serialization.
				let yaml = serde_yaml::to_string(&app).expect("Unable to serialize ImageApp.");
				let unyaml: ImageApp = serde_yaml::from_str(&yaml).expect("Unable to deserialize ImageApp.");
				assert_eq!(app, unyaml);
			},
			_ => {}
		}

		// Make sure trying fails on a bad path.
		assert_eq!(
			ImageApp::try_zopflipng(format::path::abs_pathbuf("./tests/assets/01.jpg")),
			ImageApp::None,
		);
	}

	#[test]
	#[ignore]
	/// Test ImageApp JPEG Compression.
	fn test_image_app_jpeg_compression() {
		let jpg = format::path::abs_pathbuf("./tests/assets/01.jpg");

		// Test whichever apps are available.
		for (app, slug) in vec![
			(ImageApp::find_jpegoptim(), "jpegoptim"),
			(ImageApp::find_mozjpeg(), "jpegtran"),
		].iter() {
			// If the app isn't on the system we can't test its compression.
			if *app == ImageApp::None {
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
	/// Test ImageApp PNG Compression.
	fn test_image_app_png_compression() {
		let png = format::path::abs_pathbuf("./tests/assets/02.png");

		// Test whichever apps are available.
		for (app, slug) in vec![
			(ImageApp::find_oxipng(), "oxipng"),
			(ImageApp::find_pngout(), "pngout"),
			(ImageApp::find_zopflipng(), "zopflipng"),
		].iter() {
			// If the app isn't on the system we can't test its compression.
			if *app == ImageApp::None {
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
