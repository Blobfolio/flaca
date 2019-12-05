/*!
# Formatting Helpers: Files
*/

use crate::error::Error;
use crate::image::ImageKind;
use nix::unistd::{self, Uid, Gid};
use rayon::prelude::*;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;



// ---------------------------------------------------------------------
// Conversion
// ---------------------------------------------------------------------

/// Absolute PathBuf.
///
/// For performance reasons, Rust paths do not auto-canonicalize
/// themselves. That's good and well for most use cases, but makes batch
/// operations — sort, dedup, etc. — inconsistent.
///
/// This method will always return a PathBuf. If the path exists, it
/// will be instantiated with the absolute path.
pub fn abs_pathbuf<P> (path: P) -> PathBuf
where P: AsRef<Path> {
	match path.as_ref().canonicalize() {
		Ok(path) => path,
		_ => path.as_ref().to_path_buf(),
	}
}

/// As String.
///
/// This returns the full path as a proper String.
pub fn as_string<P> (path: P) -> String
where P: AsRef<Path> {
	abs_pathbuf(path)
		.into_os_string()
		.to_str()
		.unwrap_or("")
		.to_string()
}

/// Unique File Path.
///
/// Ensure a proposed file path is unique and will not collide should
/// you decide to plop a file down there.
///
/// It is important to note that the parent directory must already
/// exist or the method will fail.
///
/// If a file already exists at this path, the name will be mutated
/// slightly to ensure uniqueness.
pub fn as_unique_pathbuf<P> (path: P) -> Result<PathBuf, Error>
where P: AsRef<Path> {
	// We can't do anything if the full path is itself a directory.
	if path.as_ref().is_dir() {
		return Err(Error::InvalidPath(as_string(&path)));
	}

	// The directory must already exist.
	let dir: PathBuf = parent_dir(&path)?;

	// We need a file name but it can be whatever.
	let name: OsString = file_name(&path);

	// If what we have is unique already, we're done!
	let mut proposed = dir.clone();
	proposed.push(&name);
	if false == proposed.exists() {
		return Ok(abs_pathbuf(&proposed));
	}

	// Let's add some uniqueness.
	for i in 0..99 {
		let mut alt_name = OsStr::new(&format!("{}--", i)).to_os_string();
		alt_name.push(&name);
		proposed = dir.clone();
		proposed.push(alt_name);
		if false == proposed.exists() {
			return Ok(abs_pathbuf(&proposed));
		}
	}

	Err(Error::new("Unable to find a unique name."))
}



// ---------------------------------------------------------------------
// Getters
// ---------------------------------------------------------------------

/// File Extension.
///
/// This returns a file's extension as a lowercase OsString. If the path
/// is a directory or has no extension, the result will be empty.
pub fn file_extension<P> (path: P) -> OsString
where P: AsRef<Path> {
	// Directories have no extension.
	if true == path.as_ref().is_dir() {
		return OsStr::new("").to_os_string();
	}

	match path.as_ref().extension() {
		Some(ext) => {
			let ext: String = ext.to_str()
				.unwrap_or("")
				.to_string()
				.to_lowercase();

			OsStr::new(&ext).to_os_string()
		},
		_ => OsStr::new("").to_os_string(),
	}
}

/// File Name.
///
/// Return the file name portion of a path as an OsString. If there
/// isn't a name, an empty string is returned.
pub fn file_name<P> (path: P) -> OsString
where P: AsRef<Path> {
	// This doesn't count for directories.
	if path.as_ref().is_dir() {
		return OsStr::new("").to_os_string();
	}

	path.as_ref()
		.file_name()
		.unwrap_or(OsStr::new(""))
		.to_os_string()
}

/// File Size.
///
/// Return the size of a file in bytes. If the path does not point to a
/// valid file, zero is returned.
pub fn file_size<P> (path: P) -> usize
where P: AsRef<Path> {
	if let Ok(meta) = path.as_ref().metadata() {
		if meta.is_file() {
			return meta.len() as usize;
		}
	}

	0
}

/// Sum File Sizes
pub fn file_sizes(paths: &Vec<PathBuf>) -> usize {
	paths.par_iter()
		.map(|ref x| file_size(&x))
		.sum()
}

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

/// Image Kind.
pub fn image_kind<P> (path: P, quick: bool) -> ImageKind
where P: AsRef<Path> {
	// If this isn't a file, we're done. Haha.
	if false == path.as_ref().is_file() {
		return ImageKind::None;
	}

	// Look deeper if we need to.
	match quick {
		true => {
			// Check the file extension first.
			let ext: String = file_extension(&path)
				.to_str()
				.unwrap_or("")
				.to_string();

			if "png" == ext {
				ImageKind::Png
			}
			else if "jpg" == ext || "jpeg" == ext {
				ImageKind::Jpeg
			}
			else {
				ImageKind::None
			}
		}
		false => match imghdr::from_file(&path) {
			Ok(Some(imghdr::Type::Png)) => ImageKind::Png,
			Ok(Some(imghdr::Type::Jpeg)) => ImageKind::Jpeg,
			_ => ImageKind::None,
		},
	}
}

/// Parent Directory.
pub fn parent_dir<P> (path: P) -> Result<PathBuf, Error>
where P: AsRef<Path> {
	let dir = path.as_ref()
		.parent()
		.ok_or(Error::InvalidPath(as_string(&path)))?;

	match dir.is_dir() {
		true => Ok(abs_pathbuf(&dir)),
		false => Err(Error::InvalidPath(as_string(&path))),
	}
}

/// Saved.
pub fn saved(before: usize, after: usize) -> usize {
	match 0 < after && after < before {
		true => before - after,
		false => 0
	}
}



// ---------------------------------------------------------------------
// Evaluation
// ---------------------------------------------------------------------

/// Has File Extension.
///
/// Run a case-insensitive check to see if a given file has a given
/// extension. The path must be valid and have an extension for this
/// to evaluate at all.
pub fn has_extension<P, S> (path: P, ext: S) -> bool
where P: AsRef<Path>, S: Into<OsString> {
	let ext = ext.into();
	let real = file_extension(&path);

	if false == real.is_empty() && false == ext.is_empty() {
		// A direct hit.
		if ext == real {
			return true;
		}

		return
			ext.to_str()
				.unwrap_or("")
				.to_string()
				.to_lowercase() ==
			real.to_str()
				.unwrap_or("")
				.to_string();
	}

	false
}

/// Is Executable.
///
/// Check whether a given file path is executable. If the path does not
/// point to a file or if that file lacks executable permissions, false
/// is returned.
pub fn is_executable<P> (path: P) -> bool
where P: AsRef<Path> {
	if let Ok(meta) = path.as_ref().metadata() {
		if meta.is_file() {
			let permissions = meta.permissions();
			return permissions.mode() & 0o111 != 0;
		}
	}

	return false;
}

/// Is Image Kind.
pub fn is_image_kind<P> (path: P, kind: ImageKind) -> bool
where P: AsRef<Path> {
	image_kind(path, false) == kind
}

/// Is Image (Period).
pub fn is_image<P> (path: P, quick: bool) -> bool
where P: AsRef<Path> {
	image_kind(path, quick) != ImageKind::None
}



// ---------------------------------------------------------------------
// I/O Operations
// ---------------------------------------------------------------------

/// Copy File.
///
/// This will copy both a file and its ownership and permission
/// settings.
///
/// The destination should be a complete file path pointing to a
/// directory that already exists. If the destination file itself
/// already exists, it will be overwritten.
pub fn copy_file<P1, P2> (from: P1, to: P2) -> Result<(), Error>
where P1: AsRef<Path>, P2: AsRef<Path> {
	// The current path must be a file, and the destination must not be
	// a directory.
	if false == from.as_ref().is_file() {
		return Err(Error::InvalidPath(as_string(&from)));
	}
	else if true == to.as_ref().is_dir() {
		return Err(Error::InvalidPath(as_string(&to)));
	}

	// The target directory must already exist too.
	parent_dir(to.as_ref())?;

	// Go ahead and copy it.
	fs::copy(&from, &to)?;

	// We should have a proper file now.
	let path: PathBuf = abs_pathbuf(&to);
	if false == path.is_file() {
		return Err(Error::IOCopy(as_string(&from), as_string(&to)));
	}

	// Make sure the permissions and ownership are correct.
	if let Ok(meta) = from.as_ref().metadata() {
		// Permissions are easy.
		if let Err(_) = fs::set_permissions(&path, meta.permissions()) {};

		// Ownership is a little more annoying.
		if let Err(_) = unistd::chown(
			&path,
			Some(Uid::from_raw(meta.uid())),
			Some(Gid::from_raw(meta.gid()))
		) {};
	}

	Ok(())
}

/// Copy File (Preserving Ownership, etc.)
///
/// This works just like `copy_file` except the ownership and
/// permissions of the destination are left as were.
///
/// Obviously both paths must exist.
pub fn copy_file_bytes<P1, P2> (from: P1, to: P2) -> Result<(), Error>
where P1: AsRef<Path>, P2: AsRef<Path> {
	// Both paths must exist and be files.
	if false == from.as_ref().is_file() {
		return Err(Error::InvalidPath(as_string(&from)));
	}
	else if false == to.as_ref().is_file() {
		return Err(Error::InvalidPath(as_string(&to)));
	}

	if to.as_ref().exists() {
		use std::fs::File;
		use std::fs::OpenOptions;
		use std::io::{prelude::*, Seek, SeekFrom};

		let mut data: Vec<u8> = Vec::with_capacity(file_size(&from));

		{
			// Read it to a buffer!
			let mut f = File::open(&from)?;
			f.read_to_end(&mut data)?;
			f.flush()?;
		}

		{
			// Now open the destination.
			let mut out = OpenOptions::new()
				.read(true)
				.write(true)
				.truncate(true)
				.open(&to)?;

			out.seek(SeekFrom::Start(0)).unwrap();
			out.write_all(&data).unwrap();
			out.flush()?;
		}

		return Ok(())
	}

	Ok(())
}


/// Delete File.
///
/// Remove a file from the file system.
pub fn delete_file<P> (path: P) -> Result<(), Error>
where P: AsRef<Path> {
	// Only for files!
	if false == path.as_ref().is_file() {
		return Err(Error::InvalidPath(as_string(&path)));
	}

	fs::remove_file(&path).map_err(|x| x.into())
}

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
						true => Some(abs_pathbuf(path)),
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
		let mut path = dir.clone();
		path.push(&name);

		if is_executable(&path) {
			return Some(abs_pathbuf(&path));
		}
	}

	None
}

/// Move File.
///
/// For a little atomicity, this method will first copy the source
/// to the destination (along with its ownership and permissions)
/// and then delete the source.
pub fn move_file<P1, P2> (from: P1, to: P2) -> Result<(), Error>
where P1: AsRef<Path>, P2: AsRef<Path> {
	// Copy first.
	copy_file(&from, &to)?;

	// Remove the original.
	delete_file(&from)?;

	// We should be good!
	Ok(())
}

/// Move File (Preserving Ownership, etc.)
///
/// This works just like move_file except the ownership and permissions
/// of the destination are left as were.
///
/// Both source and destination must exist.
pub fn move_file_bytes<P1, P2> (from: P1, to: P2) -> Result<(), Error>
where P1: AsRef<Path>, P2: AsRef<Path> {
	// Copy first.
	copy_file_bytes(&from, &to)?;

	// Remove the original.
	delete_file(&from)?;

	// We should be good!
	Ok(())
}

/// Working Copy File.
///
/// This method copies a path to a temporary location safe for
/// future meddling. The temporary location is returned unless the
/// operation failed, in which case an error is returned.
pub fn tmp_copy_file<P> (path: P) -> Result<PathBuf, Error>
where P: AsRef<Path> {
	if false == path.as_ref().is_file() {
		return Err(Error::InvalidPath(as_string(&path)));
	}

	// Build a destination path.
	let mut to: PathBuf = env::temp_dir();
	let mut stub: OsString = OsStr::new(
		&format!(
			"flaca_{}_",
			SystemTime::now().duration_since(UNIX_EPOCH)
				.unwrap_or(Duration::new(5, 0))
				.as_secs()
		)
	).to_os_string();
	stub.push(file_name(&path));
	to.push(&stub);
	to = as_unique_pathbuf(&to)?;

	// Go ahead and copy it.
	copy_file(&path, &to)?;
	Ok(abs_pathbuf(&to))
}

/// Recursive Image Walker.
pub fn walk(paths: &Vec<PathBuf>) -> Result<Vec<PathBuf>, Error> {
	// Early abort if there are no paths.
	if true == paths.is_empty() {
		return Err(Error::NoImages);
	}

	// Hold the results.
	let mut out: Vec<PathBuf> = Vec::new();

	// Loop and walk.
	for path in paths.as_parallel_slice() {
		// Recurse.
		if path.is_dir() {
			// Walk the directory.
			let walked: Vec<PathBuf> = WalkDir::new(abs_pathbuf(&path))
				.follow_links(true)
				.into_iter()
				.filter_map(|x| match x {
					Ok(path) => {
						let path = path.path();
						match is_image(&path, true) {
							true => Some(abs_pathbuf(&path)),
							false => None,
						}
					},
					_ => None,
				})
				.collect();

			if false == walked.is_empty() {
				out.par_extend(walked);
			}
		}
		// It's just a file.
		else if is_image(&path, true) {
			out.push(abs_pathbuf(&path));
		}
	}

	// If we didn't turn anything up, we're done.
	if out.is_empty() {
		return Err(Error::NoImages);
	}
	// If there is more than one result, let's make sure the list is
	// sorted and deduplicated.
	else if 1 < out.len() {
		out.par_sort();
		out.dedup();
	}

	// Done!
	Ok(out)
}



#[cfg(test)]
mod tests {
	use super::*;



	#[test]
	/// Test ABS PathBuf.
	fn test_abs_pathbuf() {
		// A good path.
		let path: PathBuf = abs_pathbuf("./src/lib.rs");
		assert!(path != PathBuf::from("./src/lib.rs"));
		assert_eq!(path, PathBuf::from("./src/lib.rs").canonicalize().unwrap());

		// A bad path.
		let path: PathBuf = abs_pathbuf("./src/library.rs");
		assert_eq!(path, PathBuf::from("./src/library.rs"));
		assert_eq!(path.canonicalize().is_ok(), false);
	}

	#[test]
	/// Test As String.
	fn test_as_string() {
		// A good path.
		let path: PathBuf = PathBuf::from("./src/lib.rs");
		assert!(as_string(path) != "./src/lib.rs");

		// A bad path.
		let path: PathBuf = PathBuf::from("./src/library.rs");
		assert_eq!(as_string(path), "./src/library.rs");
	}

	#[test]
	/// Test Make Unique.
	fn test_as_unique_pathbuf() {
		// Test a path that already exists.
		let path: PathBuf = as_unique_pathbuf("./src/lib.rs").unwrap();
		let name = file_name(&path);
		assert_eq!(name, "0--lib.rs");
		assert_eq!(parent_dir(&path), parent_dir("./src/lib.rs"));

		// Test a unique path.
		let path: PathBuf = as_unique_pathbuf("./src/library.rs").unwrap();
		let name = file_name(&path);
		assert_eq!(name, "library.rs");
		assert_eq!(parent_dir(&path), parent_dir("./src/library.rs"));

		// Test a path that is unusable.
		assert!(as_unique_pathbuf("./404/lib.rs").is_err());
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
			assert_eq!(file_extension(path), expected);
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
			assert_eq!(file_name(path), expected);
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
			assert_eq!(file_size(path), expected);
		}

		// Let's also make sure the file_sizes() method works.
		assert_eq!(
			file_sizes(&vec![PathBuf::from("./tests/assets/01.jpg"), PathBuf::from("./tests/assets/01.png")]),
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
			assert_eq!(image_kind(path, quick), expected);
		}
	}

	#[test]
	/// Test Parent.
	fn test_parent_dir() {
		// Good file, good parent.
		let path: PathBuf = PathBuf::from("./src/lib.rs");
		let parent = parent_dir(&path);
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(abs_pathbuf("./src")));

		// The parent of a directory.
		let path: PathBuf = PathBuf::from("./tests/assets");
		let parent = parent_dir(&path);
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(abs_pathbuf("./tests")));

		// Bad file, good parent.
		let path: PathBuf = PathBuf::from("./src/404.jpg");
		let parent = parent_dir(&path);
		assert!(parent.is_ok());
		assert_eq!(parent, Ok(abs_pathbuf("./src")));

		// Bad all around.
		let path: PathBuf = PathBuf::from("./404/test.jpg");
		let parent = parent_dir(&path);
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
			assert_eq!(has_extension(path, ext), expected);
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
			assert_eq!(is_executable(path), expected);
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
			assert_eq!(is_image(path, false), expected);
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
			assert_eq!(is_image_kind(path, kind), expected);
		}

		// Let's also double-check quick/slow works as expected by
		// giving a JPEG a PNG extension.
		let fake: PathBuf = abs_pathbuf("./tests/assets/wolf.png");
		assert!(fake.is_file());
		assert_eq!(image_kind(&fake, true), ImageKind::Png);
		assert_eq!(image_kind(&fake, false), ImageKind::Jpeg);
		assert!(is_image_kind(&fake, ImageKind::Jpeg));
		assert_eq!(is_image_kind(&fake, ImageKind::Png), false);

		// And again in the reverse.
		let fake: PathBuf = abs_pathbuf("./tests/assets/wolf.jpg");
		assert!(fake.is_file());
		assert_eq!(image_kind(&fake, true), ImageKind::Jpeg);
		assert_eq!(image_kind(&fake, false), ImageKind::Png);
		assert!(is_image_kind(&fake, ImageKind::Png));
		assert_eq!(is_image_kind(&fake, ImageKind::Jpeg), false);
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
	/// * tmp_copy_file()
	fn test_io_ops() {
		// Start with a valid JPEG.
		let path = PathBuf::from("./tests/assets/01.jpg");
		assert!(is_image(&path, false));

		// Make a temporary copy.
		let path2 = tmp_copy_file(&path).expect("Failed creating temporary copy.");
		assert!(is_image(&path2, false));
		assert_ne!(&path2, &path);

		// Make another temporary copy.
		let path3 = tmp_copy_file(&path).expect("Failed creating temporary copy.");
		assert!(is_image(&path3, false));
		assert_ne!(&path3, &path);
		assert_ne!(&path3, &path2);

		// The file sizes should all match too.
		assert_eq!(file_size(&path), file_size(&path2));
		assert_eq!(file_size(&path), file_size(&path3));

		// Let's prepare a path to move a file to.
		let mut path4: PathBuf = env::temp_dir();
		assert!(path4.is_dir());
		path4.push("test_io_ops.jpg");
		assert_eq!(path4.exists(), false);
		assert_eq!(parent_dir(&path4), parent_dir(&path3));

		// Actually try moving...
		assert!(move_file(&path3, &path4).is_ok());
		assert_eq!(path3.is_file(), false);
		assert_eq!(path4.is_file(), true);

		// Now let's try moving just the bytes.
		assert!(move_file_bytes(&path4, &path2).is_ok());
		assert_eq!(path4.is_file(), false);
		assert_eq!(path2.is_file(), true);

		// Moving actually tests both copy and delete actions, so we
		// should be covered there. But we still have some cleanup to do
		// so might as well redundantly test the results of that.
		assert!(delete_file(&path2).is_ok());
		assert_eq!(path2.is_file(), false);
	}

	#[test]
	/// Test Walking.
	fn test_walk() {
		// Pull test images.
		let raw = vec![PathBuf::from("./tests")];
		let paths = walk(&raw);
		assert!(paths.is_ok());
		assert_eq!(paths.unwrap().len(), 13);

		// Try running against a directory with no images.
		let raw = vec![PathBuf::from("./src")];
		let paths = walk(&raw);
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
		let paths = walk(&raw);
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
		let paths = walk(&raw);
		assert!(paths.is_ok());
		assert_eq!(paths.unwrap().len(), 13);
	}
}
