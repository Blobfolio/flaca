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

#[derive(Clone, Debug, PartialEq)]
/// An ergonomic path wrapper.
pub enum Lugar {
	Path(PathBuf),
}

impl Lugar {
	/// As path.
	pub fn path(&self) -> Result<&Path, Error> {
		match *self {
			Lugar::Path(ref p) => Ok(p.as_path()),
		}
	}

	/// Path exists?
	pub fn exists(&self) -> bool {
		match *self {
			Lugar::Path(ref p) => p.exists()
		}
	}

	/// Path is a directory?
	pub fn is_dir(&self) -> bool {
		match *self {
			Lugar::Path(ref p) => p.is_dir()
		}
	}

	/// Path is a file?
	pub fn is_file(&self) -> bool {
		match *self {
			Lugar::Path(ref p) => p.is_file()
		}
	}

	/// Canonical path.
	pub fn canonical(&self) -> Result<String, Error> {
		match *self {
			Lugar::Path(ref p) => {
				let x = p.canonicalize()?;
				Ok(format!("{}", x.display()))
			}
		}
	}

	/// File extension.
	pub fn extension(&self) -> Result<String, Error> {
		match *self {
			Lugar::Path(ref p) => {
				if let Some(x) = p.extension() {
					if let Some(y) = std::ffi::OsStr::to_str(x) {
						return Ok(y.to_string());
					}
				}

				Err(Error::new(ErrorKind::NotFound, "Could not get file name.").into())
			}
		}
	}

	/// Modification time.
	pub fn mtime(&self) -> Result<SystemTime, Error> {
		match *self {
			Lugar::Path(ref p) => {
				let x = p.metadata()?.modified()?;
				Ok(x)
			}
		}
	}

	/// Modification time relative to now.
	pub fn mtime_from_now(&self) -> Result<u64, Error> {
		let x = self.mtime()?;
		let now = SystemTime::now();
		let since = now.duration_since(x).map_err(|x| { Error::new(ErrorKind::NotFound, x) } )?;

		Ok(since.as_secs())
	}

	/// File or directory name.
	pub fn name(&self) -> Result<String, Error> {
		match *self {
			Lugar::Path(ref p) => {
				if let Some(x) = p.file_name() {
					if let Some(y) = std::ffi::OsStr::to_str(x) {
						return Ok(y.to_string());
					}
				}

				Err(Error::new(ErrorKind::NotFound, "Could not get file name.").into())
			}
		}
	}

	/// User and Group ID.
	pub fn owner(&self) -> Result<(Uid, Gid), Error> {
		match *self {
			Lugar::Path(ref p) => {
				let x = p.metadata()?;
				Ok((
					Uid::from_raw(x.uid()),
					Gid::from_raw(x.gid()),
				))
			}
		}
	}

	/// Unix permissions.
	pub fn perms(&self) -> Result<Permissions, Error> {
		match *self {
			Lugar::Path(ref p) => {
				let x = p.metadata()?;
				Ok(x.permissions())
			}
		}
	}

	/// File size in bytes.
	pub fn size(&self) -> Result<u64, Error> {
		match *self {
			Lugar::Path(ref p) => {
				let x = p.metadata()?;
				Ok(x.len())
			}
		}
	}

	/// Copy to another location.
	pub fn copy<P: AsRef<Path>>(
		&self,
		to: P,
		mut perms: Option<Permissions>,
		mut owner: Option<(Uid, Gid)>,
	) -> Result<(), Error> {
		// Our starting point has to exist, and has to be a file.
		if ! self.is_file() {
			return Err(Error::new(ErrorKind::NotFound, "Missing source file.").into())
		}

		// If TO is a file, remove it to prevent collisions.
		if to.as_ref().is_file() {
			if let Err(_) = std::fs::remove_file(to.as_ref().to_path_buf()) {
				return Err(Error::new(ErrorKind::AlreadyExists, "Destination already exists.").into())
			}
		}
		// If TO is a directory, it should be a directory plus the
		// source file name.
		else if to.as_ref().is_dir() {
			let name: String = self.name()?;
			to.as_ref().to_path_buf().push(name);
		}

		match *self {
			Lugar::Path(ref p) => {
				// First, try to copy.
				let _ = std::fs::copy(p, to.as_ref().to_path_buf())?;

				// If no permissions were specified, use the source.
				if perms.is_none() {
					if let Ok(y) = self.perms() {
						perms = Some(y);
					}
				}

				// Same with ownership.
				if owner.is_none() {
					if let Ok(y) = self.owner() {
						owner = Some(y);
					}
				}

				// Set permissions.
				if let Some(x) = perms {
					let _ = std::fs::set_permissions(to.as_ref(), x).ok();
				}

				// Set ownership.
				if let Some((x, y)) = owner {
					let _ = nix::unistd::chown(to.as_ref(), Some(x), Some(y)).ok();
				}

				Ok(())
			}
		}
	}

	/// Clone to another location.
	///
	/// This is like copy except the source is copied to a unique,
	/// temporary location.
	pub fn clone(
		&self,
		perms: Option<Permissions>,
		owner: Option<(Uid, Gid)>,
	) -> Result<Lugar, Error> {
		let ext = self.extension()?;
		let name = self.name()?;
		let dir = Lugar::tmp_dir();
		let mut num: u32 = 0;

		// Build a potential clone name.
		let mut out: String = format!(
			"{}/{}.__flaca{}.{}",
			dir,
			name,
			num.to_string(),
			ext
		);

		// If it exists, bump num until we have something new.
		while Path::new(&out).exists() {
			num += 1;

			out = format!(
				"{}/{}.__flaca{}.{}",
				dir,
				name,
				num.to_string(),
				ext
			);
		}

		// Copy!
		let _ = self.copy(&out, perms, owner)?;
		Ok(Lugar::from(out))
	}

	/// Move to another location.
	pub fn migrate<P: AsRef<Path>>(
		&self,
		to: P,
		perms: Option<Permissions>,
		owner: Option<(Uid, Gid)>,
	) -> Result<Lugar, Error> {
		// First, copy.
		let _ = self.copy(to.as_ref(), perms, owner)?;

		// Remove the original.
		if let Ok(ref x) = self.path() {
			let _ = std::fs::remove_file(x.to_path_buf()).ok();
		}

		Ok(Lugar::Path(to.as_ref().to_path_buf()))
	}

	/// Temporary directory.
	fn tmp_dir() -> String {
		lazy_static! {
			static ref dir: String = {
				let tmp = PathBuf::from(std::env::temp_dir());
				match tmp.canonicalize() {
					Ok(x) => format!("{}", x.display()),
					Err(_) => "./".to_string(),
				}
			};
		}

		dir.to_string()
	}
}

impl fmt::Display for Lugar {
	/// Display format.
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			Lugar::Path(ref p) => {
				// Prefer canonical.
				if let Ok(x) = self.canonical() {
					write!(f, "{}", x)
				}
				// But fall back to whatever bullshit was passed.
				else {
					write!(f, "{}", p.display())
				}
			}
		}
	}
}

impl<T: Into<PathBuf>> From<T> for Lugar {
	/// Into PathBuf.
	fn from(s: T) -> Self {
		Lugar::Path(s.into())
	}
}

