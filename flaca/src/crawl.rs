/*!
# Flaca: Path Crawling.
*/

use crate::{
	E_GIF,
	E_JPEG,
	E_JPG,
	E_PNG,
	FlacaError,
};
use dowser::{
	Dowser,
	Extension,
};
use std::{
	ffi::OsString,
	path::PathBuf,
};



/// # File Crawler.
///
/// This struct is a thin wrapper around `Dowser`, allowing us to defer
/// searching until all the details are in.
///
/// (`Dowser` symlink preferences need to be specified before paths, but CLI
/// argument ordering is arbitrary.)
pub(super) struct Crawler {
	/// # Follow Symlinks?
	symlinks: bool,

	/// # Path Roots.
	paths: Vec<OsString>,

	/// # List Paths.
	lists: Vec<String>,
}

impl Crawler {
	/// # New.
	///
	/// Return a new, empty instance.
	pub(super) const fn new() -> Self {
		Self {
			symlinks: true,
			paths: Vec::new(),
			lists: Vec::new(),
		}
	}

	/// # No Symlinks.
	///
	/// Symlinks are followed by default; this puts a stop to that.
	pub(super) const fn no_symlinks(&mut self) { self.symlinks = false; }

	/// # Push List.
	///
	/// Add a new list path to the queue.
	pub(super) fn push_list(&mut self, path: String) { self.lists.push(path); }

	/// # Push Path.
	///
	/// Add a new root path to the queue.
	pub(super) fn push_path(&mut self, path: OsString) { self.paths.push(path); }

	/// # Crawl!
	///
	/// Find, sort, and return all `.jpg`/`.jpeg`/`.png` files, or an error if
	/// the search comes up empty.
	///
	/// Note that this pass is name-based; the _actual_ image type for each
	/// file will be verified later.
	pub(super) fn crawl(self) -> Result<Vec<PathBuf>, FlacaError> {
		// Start the crawler.
		let mut raw = Dowser::default();

		// Disable symlinks?
		if ! self.symlinks { raw = raw.without_symlinks(); }

		// Add lists?
		for s in self.lists {
			raw.read_paths_from_file(s).map_err(|_| FlacaError::ListFile)?;
		}

		// Add paths?
		for s in self.paths { raw = raw.with_path(s); }

		// Consume!
		let mut out: Vec<PathBuf> = raw.filter(|p|
			matches!(Extension::from_path(p), Some(E_GIF | E_JPG | E_JPEG | E_PNG))
		)
			.collect();

		// Done!
		if out.is_empty() { Err(FlacaError::NoImages) }
		else {
			out.sort();
			Ok(out)
		}
	}
}
