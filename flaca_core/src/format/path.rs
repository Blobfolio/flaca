/*!
# Formatting Helpers: Files
*/

use crate::paths::{PathDisplay, PathProps};
use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};




// ---------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------

/// Human-Readable Size.
///
/// Convert a numerical byte size into a string with the best unit
/// given the value.
pub fn human_size<S> (size: S) -> String
where S: Into<usize> {
	let bytes = size.into() as f64;

	let kb: f64 = 1024.0;
	let mb: f64 = 1024.0 * 1024.0;
	let gb: f64 = 1024.0 * 1024.0 * 1024.0;

	let (bytes, unit) = if bytes > gb * 0.9 {
		(bytes / gb, "GB")
	} else if bytes > mb * 0.9 {
		(bytes / mb, "MB")
	} else if bytes > kb * 0.9 {
		(bytes / kb, "KB")
	} else {
		return format!("{}B", bytes);
	};

	format!("{:.*}{}", 2, bytes, unit)
}

/// Saved.
pub fn saved(before: usize, after: usize) -> usize {
	match 0 < after && after < before {
		true => before - after,
		false => 0
	}
}



// ---------------------------------------------------------------------
// I/O Operations
// ---------------------------------------------------------------------

/// Find Executable.
///
/// Return the executable path for a command if it exists.
pub fn find_executable<S> (name: S) -> Option<PathBuf>
where S: Into<OsString> {
	lazy_static! {
		// We only need to build a list of executable base paths once.
		static ref EXECUTABLE_DIRS: Vec<PathBuf> =
			format!("{}", env::var("PATH").unwrap_or("".to_string()))
				.split(":")
				.filter_map(|x| {
					let path = Path::new(x);
					match path.is_dir() {
						true => Some(path.flaca_to_abs_pathbuf()),
						false => None,
					}
				})
				.collect();
	}

	// If there's no $PATH, there's no binary.
	if EXECUTABLE_DIRS.is_empty() {
		return None;
	}

	let name = name.into();
	for dir in EXECUTABLE_DIRS.as_slice() {
		let path = dir.flaca_with_file_name(&name);
		if path.flaca_is_executable() {
			return Some(path);
		}
	}

	None
}



#[cfg(test)]
mod tests {
	use super::*;
	use crate::image::ImageKind;
	use crate::paths::{PathIO, PathVec};



	#[test]
	/// Test ABS PathBuf.
	fn test_abs_pathbuf() {
		// A good path.
		let path: PathBuf = PathBuf::from("./src/lib.rs").flaca_to_abs_pathbuf();
		assert!(path != PathBuf::from("./src/lib.rs"));
		assert_eq!(path, PathBuf::from("./src/lib.rs").canonicalize().unwrap());

		// A bad path.
		let path: PathBuf = PathBuf::from("./src/library.rs").flaca_to_abs_pathbuf();
		assert_eq!(path, PathBuf::from("./src/library.rs"));
		assert_eq!(path.canonicalize().is_ok(), false);
	}

	#[test]
	/// Test As String.
	fn test_as_string() {
		// A good path.
		let path: PathBuf = PathBuf::from("./src/lib.rs");
		assert!(path.flaca_to_string() != "./src/lib.rs");

		// A bad path.
		let path: PathBuf = PathBuf::from("./src/library.rs");
		assert_eq!(path.flaca_to_string(), "./src/library.rs");
	}

	#[test]
	/// Test Make Unique.
	fn test_as_unique_pathbuf() {
		// Test a path that already exists.
		let path: PathBuf = PathBuf::from("./src/lib.rs").flaca_to_unique_pathbuf().unwrap();
		let name = path.flaca_file_name();
		assert_eq!(name, "0--lib.rs");
		assert_eq!(path.flaca_parent(), PathBuf::from("./src/lib.rs").flaca_parent());

		// Test a unique path.
		let path: PathBuf = PathBuf::from("./src/library.rs").flaca_to_unique_pathbuf().unwrap();
		let name = path.flaca_file_name();
		assert_eq!(name, "library.rs");
		assert_eq!(path.flaca_parent(), PathBuf::from("./src/library.rs").flaca_parent());

		// Test a path that is unusable.
		assert!(PathBuf::from("./404/lib.rs").flaca_to_unique_pathbuf().is_err());
	}

	#[test]
	/// Test File Extension.
	fn test_file_extension() {
		let data = vec![
			("./src/404.RS", "rs"),
			("./src/404", ""),
			("./src/lib.rs", "rs"),
			("./tests/assets/01.jpg", "jpg"),
			("./tests", ""),
		];

		for d in data.as_slice() {
			let (path, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_file_extension(), expected);
		}
	}

	#[test]
	/// Test File Name.
	fn test_file_name() {
		let data = vec![
			("./src/404.RS", "404.RS"),
			("./src/404", "404"),
			("./src/lib.rs", "lib.rs"),
			("./tests/assets/01.jpg", "01.jpg"),
			("./tests", ""),
		];

		for d in data.as_slice() {
			let (path, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_file_name(), expected);
		}
	}

	#[test]
	/// Test File Size.
	fn test_file_size() {
		let data = vec![
			("./src/404.jpg", 0),
			("./tests/assets/01.jpg", 386663),
			("./tests/assets/01.png", 211427),
			("./tests", 0),
		];

		for d in data.as_slice() {
			let (path, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_file_size(), expected);
		}

		// Let's also make sure the file_sizes() method works.
		assert_eq!(
			vec![PathBuf::from("./tests/assets/01.jpg"), PathBuf::from("./tests/assets/01.png")].flaca_file_sizes(),
			386663 + 211427
		);
	}

	#[test]
	/// Test Human Size.
	fn test_human_size() {
		let data = vec![
			(500, "500B"),
			(1024, "1.00KB"),
			(905521, "884.30KB"),
			(9055213, "8.64MB"),
		];

		for d in data.as_slice() {
			let (size, expected) = *d;
			assert_eq!(human_size(size as usize), expected);
		}
	}

	#[test]
	/// Test Image Kind.
	fn test_image_kind() {
		let data = vec![
			("./src/404.jpg", false, ImageKind::None),
			("./tests/assets/01.jpg", false, ImageKind::Jpeg),
			("./tests/assets/01.png", false, ImageKind::Png),
			("./tests", false, ImageKind::None),
		];

		for d in data.as_slice() {
			let (path, quick, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_image_kind(quick), expected);
		}
	}

	#[test]
	/// Test Parent.
	fn test_parent_dir() {
		// Good file, good parent.
		let path: PathBuf = PathBuf::from("./src/lib.rs");
		let parent = path.flaca_parent();
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(PathBuf::from("./src").flaca_to_abs_pathbuf()));

		// The parent of a directory.
		let path: PathBuf = PathBuf::from("./tests/assets");
		let parent = path.flaca_parent();
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(PathBuf::from("./tests").flaca_to_abs_pathbuf()));

		// Bad file, good parent.
		let path: PathBuf = PathBuf::from("./src/404.jpg");
		let parent = path.flaca_parent();
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(PathBuf::from("./src").flaca_to_abs_pathbuf()));

		// Bad all around.
		let path: PathBuf = PathBuf::from("./404/test.jpg");
		let parent = path.flaca_parent();
		assert_eq!(parent.is_ok(), false);
	}

	#[test]
	/// Test Has Extension.
	fn test_has_extension() {
		let data = vec![
			("./src/404.jpg", "jpg", true),
			("./tests/assets/01.jpg", "jpg", true),
			("./tests/assets/01.png", "png", true),
			("./tests/assets/01.png", "PNG", true),
			("./tests/assets/01.png", "jpg", false),
			("./tests", "rs", false),
		];

		for d in data.as_slice() {
			let (path, ext, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_has_extension(ext), expected);
		}
	}

	#[test]
	/// Test Has Extension.
	fn test_is_executable() {
		let data = vec![
			("./src/", false),
			("./tests/assets/01.jpg", false),
			("./tests/assets/404.exe", false),
			("./tests/assets/executable.sh", true),
		];

		for d in data.as_slice() {
			let (path, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_is_executable(), expected);
		}
	}

	#[test]
	/// Test Has Extension.
	fn test_is_image() {
		let data = vec![
			("./src/", false),
			("./tests/assets/01.jpg", true),
			("./tests/assets/01.png", true),
			("./tests/assets/404.exe", false),
			("./tests/assets/executable.sh", false),
		];

		for d in data.as_slice() {
			let (path, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_is_image(false), expected);
		}
	}

	#[test]
	/// Test Has Extension.
	fn test_is_image_kind() {
		let data = vec![
			("./src/", ImageKind::Jpeg, false),
			("./tests/assets/01.jpg", ImageKind::Jpeg, true),
			("./tests/assets/01.jpg", ImageKind::Png, false),
			("./tests/assets/01.png", ImageKind::Jpeg, false),
			("./tests/assets/01.png", ImageKind::Png, true),
			("./tests/assets/404.exe", ImageKind::Jpeg, false),
			("./tests/assets/executable.sh", ImageKind::Jpeg, false),
		];

		for d in data.as_slice() {
			let (path, kind, expected) = *d;
			assert_eq!(PathBuf::from(path).flaca_is_image_kind(kind), expected);
		}

		// Let's also double-check quick/slow works as expected by
		// giving a JPEG a PNG extension.
		let fake: PathBuf = PathBuf::from("./tests/assets/wolf.png").flaca_to_abs_pathbuf();
		assert!(fake.is_file());
		assert_eq!(fake.flaca_image_kind(true), ImageKind::Png);
		assert_eq!(fake.flaca_image_kind(false), ImageKind::Jpeg);
		assert!(fake.flaca_is_image_kind(ImageKind::Jpeg));
		assert_eq!(fake.flaca_is_image_kind(ImageKind::Png), false);

		// And again in the reverse.
		let fake: PathBuf = PathBuf::from("./tests/assets/wolf.jpg").flaca_to_abs_pathbuf();
		assert!(fake.is_file());
		assert_eq!(fake.flaca_image_kind(true), ImageKind::Jpeg);
		assert_eq!(fake.flaca_image_kind(false), ImageKind::Png);
		assert!(fake.flaca_is_image_kind(ImageKind::Png));
		assert_eq!(fake.flaca_is_image_kind(ImageKind::Jpeg), false);
	}

	#[test]
	/// Test Find Executable.
	fn test_find_executable() {
		let data = vec!["jpegoptim", "oxipng", "pngout", "zopflipng"];
		for d in data {
			if find_executable(d).is_some() {
				// We just want to know that we found *something*.
				assert!(true);
				return;
			}
		}
	}

	#[test]
	/// Test I/O Operations.
	///
	/// Among other things, this covers:
	/// * copy_file()
	/// * delete_file()
	/// * move_file()
	/// * copy_tmp()
	fn test_io_ops() {
		// Start with a valid JPEG.
		let path = PathBuf::from("./tests/assets/01.jpg");
		assert!(path.flaca_is_image(false));

		// Make a temporary copy.
		let path2 = path.flaca_copy_tmp().expect("Failed creating temporary copy.");
		assert!(path2.flaca_is_image(false));
		assert_ne!(&path2, &path);

		// Make another temporary copy.
		let path3 = path.flaca_copy_tmp().expect("Failed creating temporary copy.");
		assert!(path3.flaca_is_image(false));
		assert_ne!(&path3, &path);
		assert_ne!(&path3, &path2);

		// The file sizes should all match too.
		assert_eq!(path.flaca_file_size(), path2.flaca_file_size());
		assert_eq!(path.flaca_file_size(), path3.flaca_file_size());

		// Let's prepare a path to move a file to.
		let mut path4: PathBuf = env::temp_dir();
		assert!(path4.is_dir());
		path4.push("test_io_ops.jpg");
		assert_eq!(path4.exists(), false);
		assert_eq!(&path4.flaca_parent(), &path3.flaca_parent());

		// Actually try moving...
		assert!(path3.flaca_move_file(&path4).is_ok());
		assert_eq!(path3.is_file(), false);
		assert_eq!(path4.is_file(), true);

		// Now let's try moving just the bytes.
		assert!(path4.flaca_move_bytes(&path2).is_ok());
		assert_eq!(path4.is_file(), false);
		assert_eq!(path2.is_file(), true);

		// Moving actually tests both copy and delete actions, so we
		// should be covered there. But we still have some cleanup to do
		// so might as well redundantly test the results of that.
		assert!(path2.flaca_delete_file().is_ok());
		assert_eq!(path2.is_file(), false);
	}

	#[test]
	/// Test Walking.
	fn test_walk() {
		// Pull test images.
		let raw = vec![PathBuf::from("./tests")];
		let paths = raw.flaca_walk();
		assert!(paths.is_ok());
		assert_eq!(paths.unwrap().len(), 13);

		// Try running against a directory with no images.
		let raw = vec![PathBuf::from("./src")];
		let paths = raw.flaca_walk();
		assert!(paths.is_err());

		// Try with some direct image paths, including an invalid one
		// and a duplicate one.
		let raw = vec![
			PathBuf::from("./tests/assets/404.jpg"),
			PathBuf::from("./tests/assets/01.jpg"),
			PathBuf::from("./tests/assets/02.jpg"),
			PathBuf::from("./tests/assets/03.jpg"),
			PathBuf::from("./tests/assets/03.jpg"),
		];
		let paths = raw.flaca_walk();
		assert!(paths.is_ok());
		assert_eq!(paths.unwrap().len(), 3);

		// One last check for duplicate handling where multiple
		// directories are specified.
		let raw = vec![
			PathBuf::from("./tests/assets/404.jpg"),
			PathBuf::from("./tests/assets/01.jpg"),
			PathBuf::from("./tests/assets"),
			PathBuf::from("./tests"),
		];
		let paths = raw.flaca_walk();
		assert!(paths.is_ok());
		assert_eq!(paths.unwrap().len(), 13);
	}
}
