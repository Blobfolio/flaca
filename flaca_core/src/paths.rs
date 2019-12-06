/*!
# Paths
*/

use crate::error::Error;
use crate::format::strings;
use crate::format::time;
use crate::image::ImageKind;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};



/// Path Conversion/Display.
pub trait PathDisplay {
	/// Absolute PathBuf.
	fn flaca_to_abs_pathbuf(&self) -> PathBuf;

	/// To String.
	fn flaca_to_string(&self) -> String;

	/// Unique File Path.
	fn flaca_to_unique_pathbuf(&self) -> Result<PathBuf, Error>;

	/// With File Name.
	fn flaca_with_file_name<S> (&self, name: S) -> PathBuf where S: Into<OsString>;
}



/// Better Path Properties.
pub trait PathProps {
	/// File Extension.
	fn flaca_file_extension(&self) -> OsString;

	/// File Name.
	fn flaca_file_name(&self) -> OsString;

	/// File Size.
	fn flaca_file_size(&self) -> usize;

	/// Image Kind.
	fn flaca_image_kind(&self, quick: bool) -> ImageKind;

	/// Parent Directory.
	fn flaca_parent(&self) -> Result<PathBuf, Error>;

	/// Has Extension?
	fn flaca_has_extension<S> (&self, ext: S) -> bool where S: Into<OsString>;

	/// Is Executable?
	fn flaca_is_executable(&self) -> bool;

	/// Is Image?
	fn flaca_is_image(&self, quick: bool) -> bool;

	/// Is Image Kind?
	fn flaca_is_image_kind(&self, kind: ImageKind) -> bool;
}



/// Better Path IO.
pub trait PathIO {
	/// Copy File Bytes.
	fn flaca_copy_bytes<P> (&self, to: P) -> Result<(), Error> where P: AsRef<Path>;

	/// Copy File.
	fn flaca_copy_file<P> (&self, to: P) -> Result<(), Error> where P: AsRef<Path>;

	/// Temporary Copy.
	fn flaca_copy_tmp(&self) -> Result<PathBuf, Error>;

	/// Delete File.
	fn flaca_delete_file(&self) -> Result<(), Error>;

	/// Move File Bytes.
	fn flaca_move_bytes<P> (&self, to: P) -> Result<(), Error> where P: AsRef<Path>;

	/// Move File.
	fn flaca_move_file<P> (&self, to: P) -> Result<(), Error> where P: AsRef<Path>;
}



impl PathDisplay for Path {
	/// Absolute PathBuf.
	///
	/// For performance reasons, Rust paths do not auto-canonicalize
	/// themselves. That's good and well for most use cases, but makes batch
	/// operations — sort, dedup, etc. — inconsistent.
	///
	/// This method will always return a PathBuf. If the path exists, it
	/// will be instantiated with the absolute path.
	fn flaca_to_abs_pathbuf(&self) -> PathBuf {
		match self.canonicalize() {
			Ok(path) => path,
			_ => self.to_path_buf(),
		}
	}

	/// To String.
	///
	/// This returns the full path as a proper String.
	fn flaca_to_string(&self) -> String {
		strings::from_os_string(
			self.flaca_to_abs_pathbuf().into_os_string()
		)
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
	fn flaca_to_unique_pathbuf(&self) -> Result<PathBuf, Error> {
		// We can't do anything if the full path is itself a directory.
		if self.is_dir() {
			return Err(Error::InvalidPath(self.flaca_to_string()));
		}

		// The directory must already exist.
		let dir: PathBuf = self.flaca_parent()?;

		// We need a file name but it can be whatever.
		let name: OsString = self.flaca_file_name();

		// If what we have is unique already, we're done!
		let proposed = dir.flaca_with_file_name(&name);
		if false == proposed.exists() {
			return Ok(proposed);
		}

		// Let's add some uniqueness.
		for i in 0..99 {
			let alt_path = dir.flaca_with_file_name(
				format!(
					"{}--{}",
					i,
					strings::from_os_string(&name)
				)
			);
			if false == alt_path.exists() {
				return Ok(alt_path);
			}
		}

		Err(Error::new("Unable to find a unique name."))
	}

	/// With File Name.
	///
	/// This will push a name on top of a directory path, or use
	/// `::set_file_name` if the destination does not exist.
	fn flaca_with_file_name<S> (&self, name: S) -> PathBuf
	where S: Into<OsString> {
		let mut path: PathBuf = self.flaca_to_abs_pathbuf();
		match path.is_dir() {
			true => path.push(name.into()),
			false => path.set_file_name(name.into()),
		}

		path
	}
}



impl PathProps for Path {
	/// File Extension.
	///
	/// This returns a file's extension as a lowercase OsString. If the path
	/// is a directory or has no extension, the result will be empty.
	fn flaca_file_extension(&self) -> OsString {
		// Directories have no extension.
		if true == self.is_dir() {
			return strings::to_os_string("");
		}

		match self.extension() {
			Some(ext) => strings::to_os_string(
				strings::from_os_string(ext).to_lowercase()
			),
			_ => strings::to_os_string(""),
		}
	}

	/// File Name.
	///
	/// Return the file name portion of a path as an OsString. If there
	/// isn't a name, an empty string is returned.
	fn flaca_file_name(&self) -> OsString {
		// This doesn't count for directories.
		if self.is_dir() {
			return strings::to_os_string("");
		}

		self.file_name()
			.unwrap_or(OsStr::new(""))
			.to_os_string()
	}

	/// File Size.
	///
	/// Return the size of a file in bytes. If the path does not point to a
	/// valid file, zero is returned.
	fn flaca_file_size(&self) -> usize {
		if let Ok(meta) = self.metadata() {
			if meta.is_file() {
				return meta.len() as usize;
			}
		}

		0
	}

	/// Image Kind.
	fn flaca_image_kind(&self, quick: bool) -> ImageKind {
		// If this isn't a file, we're done. Haha.
		if false == self.is_file() {
			return ImageKind::None;
		}

		// Look deeper if we need to.
		match quick {
			// Quick Mode: Trust the file extension.
			true => match self.flaca_file_extension().to_str().unwrap_or("") {
				"png" => ImageKind::Png,
				"jpg" | "jpeg" => ImageKind::Jpeg,
				_ => ImageKind::None,
			},
			// Better Mode: Look at the Magic Headers.
			false => match imghdr::from_file(self.to_path_buf()) {
				Ok(Some(imghdr::Type::Png)) => ImageKind::Png,
				Ok(Some(imghdr::Type::Jpeg)) => ImageKind::Jpeg,
				_ => ImageKind::None,
			},
		}
	}

	/// Parent Directory.
	fn flaca_parent(&self) -> Result<PathBuf, Error> {
		let dir = self.parent()
			.ok_or(Error::InvalidPath(self.flaca_to_string()))?;

		match dir.is_dir() {
			true => Ok(dir.flaca_to_abs_pathbuf()),
			false => Err(Error::InvalidPath(self.flaca_to_string())),
		}
	}

	/// Has File Extension.
	///
	/// Run a case-insensitive check to see if a given file has a given
	/// extension. The path must be valid and have an extension for this
	/// to evaluate at all.
	fn flaca_has_extension<S> (&self, ext: S) -> bool
	where S: Into<OsString> {
		let ext = ext.into();
		let real = self.flaca_file_extension();

		if false == real.is_empty() && false == ext.is_empty() {
			// A direct hit.
			if ext == real {
				return true;
			}

			// Change the case and try again.
			return strings::to_os_string(strings::from_os_string(ext).to_lowercase()) == real;
		}

		false
	}

	/// Is Executable.
	///
	/// Check whether a given file path is executable. If the path does not
	/// point to a file or if that file lacks executable permissions, false
	/// is returned.
	fn flaca_is_executable(&self) -> bool {
		if let Ok(meta) = self.metadata() {
			if meta.is_file() {
				let permissions = meta.permissions();
				return permissions.mode() & 0o111 != 0;
			}
		}

		return false;
	}

	/// Is Image (Period).
	fn flaca_is_image(&self, quick: bool) -> bool {
		self.flaca_image_kind(quick) != ImageKind::None
	}

	/// Is Image Kind.
	fn flaca_is_image_kind(&self, kind: ImageKind) -> bool {
		self.flaca_image_kind(false) == kind
	}
}



impl PathIO for Path {
	/// Copy File (Preserving Ownership, etc.)
	///
	/// This works just like `copy_file` except the ownership and
	/// permissions of the destination are left as were.
	///
	/// Obviously both paths must exist.
	fn flaca_copy_bytes<P> (&self, to: P) -> Result<(), Error>
	where P: AsRef<Path> {
		// Both paths must exist and be files.
		if false == self.is_file() {
			return Err(Error::InvalidPath(self.flaca_to_string()));
		}
		else if false == to.as_ref().is_file() {
			return Err(Error::InvalidPath(to.as_ref().flaca_to_string()));
		}

		use std::fs::File;
		use std::fs::OpenOptions;
		use std::io::{prelude::*, Seek, SeekFrom};

		let mut data: Vec<u8> = Vec::with_capacity(self.flaca_file_size());

		{
			// Read it to a buffer!
			let mut f = File::open(&self)?;
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

		Ok(())
	}

	/// Copy File.
	///
	/// This will copy both a file and its ownership and permission
	/// settings.
	///
	/// The destination should be a complete file path pointing to a
	/// directory that already exists. If the destination file itself
	/// already exists, it will be overwritten.
	fn flaca_copy_file<P> (&self, to: P) -> Result<(), Error>
	where P: AsRef<Path> {
		// The current path must be a file, and the destination must not be
		// a directory.
		if false == self.is_file() {
			return Err(Error::InvalidPath(self.flaca_to_string()));
		}
		else if true == to.as_ref().is_dir() {
			return Err(Error::InvalidPath(to.as_ref().flaca_to_string()));
		}

		// The target directory must already exist too.
		to.as_ref().flaca_parent()?;

		// Go ahead and copy it.
		fs::copy(&self, &to)?;

		// We should have a proper file now.
		let path: PathBuf = to.as_ref().flaca_to_abs_pathbuf();
		if false == path.is_file() {
			return Err(Error::IOCopy(self.flaca_to_string(), to.as_ref().flaca_to_string()));
		}

		// Make sure the permissions and ownership are correct.
		if let Ok(meta) = self.metadata() {
			use nix::unistd::{self, Uid, Gid};

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

	/// Working Copy File.
	///
	/// This method copies a path to a temporary location safe for
	/// future meddling. The temporary location is returned unless the
	/// operation failed, in which case an error is returned.
	fn flaca_copy_tmp(&self) -> Result<PathBuf, Error> {
		if false == self.is_file() {
			return Err(Error::InvalidPath(self.flaca_to_string()));
		}

		// Build a destination path.
		let mut to: PathBuf = env::temp_dir();
		let stub: String = format!(
			"flaca_{}_{}",
			time::unixtime(),
			strings::from_os_string(self.flaca_file_name())
		);
		to.push(&stub);
		to = to.flaca_to_unique_pathbuf()?;

		// Go ahead and copy it.
		self.flaca_copy_file(&to)?;
		Ok(to)
	}

	/// Delete File.
	///
	/// Remove a file from the file system.
	fn flaca_delete_file(&self) -> Result<(), Error> {
		// Only for files!
		if false == self.is_file() {
			return Err(Error::InvalidPath(self.flaca_to_string()));
		}

		fs::remove_file(&self).map_err(|x| x.into())
	}

	/// Move File (Preserving Ownership, etc.)
	///
	/// This works just like move_file except the ownership and permissions
	/// of the destination are left as were.
	///
	/// Both source and destination must exist.
	fn flaca_move_bytes<P> (&self, to: P) -> Result<(), Error>
	where P: AsRef<Path> {
		// Copy first.
		self.flaca_copy_bytes(&to)?;

		// Remove the original.
		self.flaca_delete_file()?;

		// We should be good!
		Ok(())
	}

	/// Move File.
	///
	/// For a little atomicity, this method will first copy the source
	/// to the destination (along with its ownership and permissions)
	/// and then delete the source.
	fn flaca_move_file<P> (&self, to: P) -> Result<(), Error>
	where P: AsRef<Path> {
		// Copy first.
		self.flaca_copy_file(&to)?;

		// Remove the original.
		self.flaca_delete_file()?;

		// We should be good!
		Ok(())
	}
}
