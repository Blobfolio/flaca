// Flaca: Lugar
//
// A simple wrapper to make path-handling more ergonomical.
//
// ©2018 Blobfolio, LLC <hello@blobfolio.com>
// License: WTFPL <http://www.wtfpl.net>

use nix::unistd::{Uid, Gid};
use std::fmt;
use std::fs::Permissions;
use std::io::{Error, ErrorKind};
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use std::ffi::OsStr;
use chrono::{Local, DateTime, TimeZone};

/// Placeholder for empty strings.
const NOWHERE: &str = "<MISSING>";

#[derive(Debug, PartialEq, Clone)]
/// Ergonic path wrapper.
pub enum Lugar {
	Jpg(PathBuf),
	Png(PathBuf),
	Other(PathBuf),
}

impl Lugar {
	// -----------------------------------------------------------------
	// Init/Conversion
	// -----------------------------------------------------------------

	/// Create a new instance.
	pub fn new<P: AsRef<Path>>(path: P) -> Lugar {
		// These get filed according to type.
		let ext: String = {
			if let Some(x) = path.as_ref().extension() {
				if let Some(y) = OsStr::to_str(x) {
					y.to_string().to_lowercase().into()
				}
				else {
					"".to_string()
				}
			}
			else {
				"".to_string()
			}
		};

		if "jpg" == ext || "jpeg" == ext {
			Lugar::Jpg(path.as_ref().to_path_buf())
		}
		else if "png" == ext {
			Lugar::Png(path.as_ref().to_path_buf())
		}
		else {
			Lugar::Other(path.as_ref().to_path_buf())
		}
	}

	/// As Path.
	pub fn as_path(&self) -> &Path {
		self.as_path_buf().as_path()
	}

	/// As PathBuf.
	pub fn as_path_buf(&self) -> &PathBuf {
		self.__inner()
	}

	/// As String.
	pub fn as_string(&self) -> String {
		self.canonical().unwrap_or(NOWHERE.to_string())
	}

	// -----------------------------------------------------------------
	// State
	// -----------------------------------------------------------------

	/// Case-insensitive match-check for file extension.
	pub fn has_ext(&self, ext: String) -> bool {
		if let Ok(ext2) = self.extension() {
			return ext2.to_lowercase() == ext.to_lowercase();
		}

		false
	}

	/// Case-insensitive match-check for file name.
	pub fn has_name(&self, name: String) -> bool {
		if let Ok(name2) = self.name() {
			return name2.to_lowercase() == name.to_lowercase();
		}

		false
	}

	/// Does the path exist?
	pub fn is(&self) -> bool {
		self.__inner().exists()
	}

	/// Is the path a directory?
	pub fn is_dir(&self) -> bool {
		self.__inner().is_dir()
	}

	/// Is the path a file?
	pub fn is_file(&self) -> bool {
		self.__inner().is_file()
	}

	/// Is the path an image?
	pub fn is_image(&self) -> bool {
		match *self {
			Lugar::Jpg(_) => self.is_jpg(),
			Lugar::Png(_) => self.is_png(),
			_ => false,
		}
	}

	/// Is the path a JPEG image?
	pub fn is_jpg(&self) -> bool {
		match *self {
			Lugar::Jpg(ref p) => p.is_file() &&
				(self.has_ext("jpg".to_string()) || self.has_ext("jpeg".to_string())),
			_ => false,
		}
	}

	/// Is the path a PNG image?
	pub fn is_png(&self) -> bool {
		match *self {
			Lugar::Png(ref p) => p.is_file() && self.has_ext("png".to_string()),
			_ => false,
		}
	}

	// -----------------------------------------------------------------
	// Data
	// -----------------------------------------------------------------

	/// Return the inner PathBuf.
	pub fn __inner(&self) -> &PathBuf {
		match *self {
			Lugar::Jpg(ref p) => p,
			Lugar::Png(ref p) => p,
			Lugar::Other(ref p) => p,
		}
	}

	/// Return the inner PathBuf mutably.
	fn __inner_mut(&mut self) -> &mut PathBuf {
		match *self {
			Lugar::Jpg(ref mut p) => p,
			Lugar::Png(ref mut p) => p,
			Lugar::Other(ref mut p) => p,
		}
	}

	/// Canonical path.
	pub fn canonical(&self) -> Result<String, Error> {
		let x = self.__inner().canonicalize()?;
		Ok(format!("{}", x.display()))
	}

	/// File extension.
	pub fn extension(&self) -> Result<String, Error> {
		if let Some(x) = self.__inner().extension() {
			if let Some(y) = OsStr::to_str(x) {
				return Ok(y.to_string());
			}
		}

		Err(Error::new(ErrorKind::NotFound, "Could not get file name.").into())
	}

	/// Modification time.
	pub fn mtime(&self) -> Result<SystemTime, Error> {
		let x = self.__inner().metadata()?.modified()?;
		Ok(x)
	}

	/// Modification time relative to now.
	pub fn age(&self) -> Result<u64, Error> {
		let to = self.mtime()?;
		let x = Lugar::time_diff(SystemTime::now(), to)?;
		Ok(x)
	}

	/// File or directory name.
	pub fn name(&self) -> Result<String, Error> {
		if let Some(x) = self.__inner().file_name() {
			if let Some(y) = OsStr::to_str(x) {
				return Ok(y.to_string());
			}
		}

		Err(Error::new(ErrorKind::NotFound, "Could not get file name.").into())
	}

	/// User and Group ID.
	pub fn owner(&self) -> Result<(Uid, Gid), Error> {
		let x = self.__inner().metadata()?;
		Ok((
			Uid::from_raw(x.uid()),
			Gid::from_raw(x.gid()),
		))
	}

	/// Unix permissions.
	pub fn perms(&self) -> Result<Permissions, Error> {
		let x = self.__inner().metadata()?;
		Ok(x.permissions())
	}

	/// File size in bytes.
	pub fn size(&self) -> Result<u64, Error> {
		let x = self.__inner().metadata()?;
		Ok(x.len())
	}

	// -----------------------------------------------------------------
	// Operations
	// -----------------------------------------------------------------

	/// Copy a file to another location.
	pub fn cp(
		&self,
		to: &mut Lugar,
		mut perms: Option<Permissions>,
		mut owner: Option<(Uid, Gid)>
	) -> Result<(), Error> {
		// The source has to be a file.
		if ! self.is_file() {
			return Err(Error::new(ErrorKind::InvalidInput, "Source is not a file.").into());
		}

		// If the destination is a directory, we need to add the source
		// file name to the path.
		if to.is_dir() {
			let _ = to.push(self.name()?)?;
		}

		// If destination is a file, remove it to work around collision
		// errors.
		if to.is_file() {
			let _ = to.rm()?;
		}

		// Copy and update the struct, just in case it needs
		// reclassification.
		let _ = std::fs::copy(self.__inner(), to.__inner())?;
		to.set_path(to.__inner().to_path_buf())?;

		// If no permissions were specified, use the source's.
		if perms.is_none() {
			if let Ok(x) = self.perms() {
				perms = Some(x);
			}
		}
		// Set permissions.
		if let Some(x) = perms {
			if let Err(_) = to.set_perms(x) {}
		}

		// If no owner was specified, use the source's.
		if owner.is_none() {
			if let Ok(x) = self.owner() {
				owner = Some(x);
			}
		}
		// Set permissions.
		if let Some((x, y)) = owner {
			if let Err(_) = to.set_owner(x, y) {}
		}

		Ok(())
	}

	/// Move a file to somewhere else.
	pub fn mv(
		&mut self,
		to: &mut Lugar,
		perms: Option<Permissions>,
		owner: Option<(Uid, Gid)>,
	) -> Result<(), Error> {
		// First, copy.
		let _ = self.cp(to, perms, owner)?;

		// Remove the original.
		let _ = self.rm()?;

		// Replace the instance.
		*self = Lugar::new(to.__inner().to_path_buf());

		Ok(())
	}

	/// Append a sub-path to the current path.
	pub fn push<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
		self.__inner_mut().push(path.as_ref());
		*self = Lugar::new(self.__inner().to_path_buf());
		Ok(())
	}

	/// Remove a file.
	pub fn rm(&self) -> Result<(), Error> {
		if ! self.is_file() {
			return Err(Error::new(ErrorKind::InvalidInput, "Source is not a file.").into());
		}

		std::fs::remove_file(self.__inner())
	}

	/// Set a file's ownership.
	pub fn set_owner(&self, uid: Uid, gid: Gid) -> Result<(), Error> {
		if let Ok(_) = nix::unistd::chown(self.__inner(), Some(uid), Some(gid)) {
			return Ok(());
		}

		Err(Error::new(ErrorKind::Other, "Unable to set permissions.").into())
	}

	/// Update the instance path.
	pub fn set_path<P: AsRef<Path>>(&mut self, path: P) -> Result<(), Error> {
		*self = Lugar::new(path.as_ref());
		Ok(())
	}

	/// Set a file's permissions.
	pub fn set_perms(&self, perms: Permissions) -> Result<(), Error> {
		if let Ok(_) = std::fs::set_permissions(self.__inner(), perms) {
			return Ok(());
		}

		Err(Error::new(ErrorKind::Other, "Unable to set permissions.").into())
	}

	/// Clone to unique temporary file.
	///
	/// Compression operations are run on these clones instead of the
	/// source files for atomicity.
	pub fn tmp_cp(&self) -> Result<(Lugar), Error> {
		// The file name.
		let name: String = self.name()?;

		// Grab the file extension so our suffixed clone can have the
		// right extension.
		let mut ext: String = self.extension()?.to_lowercase();
		if "jpeg" == ext {
			ext = "jpg".to_string();
		}

		// Try this combination first for uniqueness.
		let mut out: Lugar = Lugar::tmp_file(format!(
			"{}.__flaca.{}",
			name,
			ext,
		));

		// If we collide with something, bump the num.
		let mut num: u8 = 0;
		while out.is() {
			num += 1;

			out = Lugar::tmp_file(format!(
				"{}.__flaca{}.{}",
				name,
				num,
				ext,
			));
		}

		// Copy this to that, then return that.
		let _ = self.cp(&mut out, None, None)?;

		Ok(out)
	}

	// -----------------------------------------------------------------
	// Misc Helpers
	// -----------------------------------------------------------------

	/// User $PATH directories.
	pub fn bin_dirs() -> Vec<Lugar> {
		lazy_static! {
			static ref paths: Vec<String> = {
				// Start with the Flaca shared directory, if it exists.
				let prefix =
					if Lugar::new("/usr/share/flaca").is_dir() {
						"/usr/share/flaca:"
					}
					else {
						""
					};

				// Pull the $PATH env var.
				format!("{}{}", prefix, std::env::var("PATH").unwrap_or("".to_string()))
					.split(":")
					.map(String::from)
					.collect()
			};
		}

		paths.iter().map(Lugar::new).collect()
	}

	/// Get a local datetime.
	pub fn local_now() -> DateTime<Local> {
		let since = Lugar::time_diff(SystemTime::now(), std::time::UNIX_EPOCH).expect("Time is meaningless.");
		Local.timestamp(since as i64, 0)
	}

	/// Find the difference between two times in seconds.
	pub fn time_diff(one: SystemTime, two: SystemTime) -> Result<u64, Error> {
		if one > two {
			let since = one.duration_since(two).map_err(|x| { Error::new(ErrorKind::Other, x) } )?;
			Ok(since.as_secs())
		}
		else {
			let since = two.duration_since(one).map_err(|x| { Error::new(ErrorKind::Other, x) } )?;
			Ok(since.as_secs())
		}
	}

	/// Temporary directory.
	pub fn tmp_dir() -> Lugar {
		Lugar::new(std::env::temp_dir())
	}

	/// Temporary file.
	pub fn tmp_file(name: String) -> Lugar {
		let mut path = Lugar::tmp_dir();
		if let Err(_) = path.push(name) {}
		path
	}
}

impl fmt::Display for Lugar {
	/// Display format.
	///
	/// This uses the canonical path when possible, but falls back to
	/// whatever was used to seed the PathBuf.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		// Prefer canonical path.
		if let Ok(x) = self.canonical() {
			write!(f, "{}", x)
		}
		// Use PathBuf display if needed.
		else {
			write!(f, "{}", self.__inner().display())
		}
	}
}

impl<T: Into<PathBuf>> From<T> for Lugar {
	/// Into PathBuf.
	fn from(s: T) -> Self {
		Lugar::new(s.into())
	}
}
