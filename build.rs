/*!
# Flaca - Build
*/

use std::{
	ffi::OsStr,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};

macro_rules! cmd {
	($cmd:expr, $oops:literal) => (
		assert!(
			$cmd
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.status()
				.map_or(false, |s| s.success()),
			$oops
		);
	);
}

/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to
/// build the standalone binary so it can be called by Flaca.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	// Local repo path.
	let repo = out_path("zopfli-git");

	// If the folder already exists, nuke it.
	if repo.is_dir() {
		std::fs::remove_dir_all(&repo).expect("Unable to clear old repo.");
	}

	// Clone the repository.
	cmd!(
		Command::new("git")
			.args(&[
				OsStr::new("clone"),
				OsStr::new("https://github.com/google/zopfli"),
				repo.as_os_str(),
			]),
		"Unable to clone Zopfli repo."
	);

	// Checkout a specific commit for reproducibility.
	cmd!(
		Command::new("git").current_dir(&repo).args(&["checkout", "831773b"]),
		"Unable to checkout Zopfli repo."
	);

	// Apply the patch so we can build the binary with LTO.
	cmd!(
		Command::new("git")
			.current_dir(&repo)
			.args(&[
				OsStr::new("apply"),
				in_path("skel/zopfli.patch").as_os_str(),
			]),
		"Unable to patch Zopfli."
	);

	// Build it.
	cmd!(
		Command::new("make").current_dir(&repo).args(&["zopflipng"]),
		"Unable to build Zopflipng."
	);

	// The bin should exist now.
	let bin = out_path("zopfli-git/zopflipng");
	assert!(bin.is_file(), "Missing built Zopflipng executable.");

	// Strip it, but if this fails that's all right.
	let _res = Command::new("strip")
		.args(&[bin.as_os_str()])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status();

	// Last check, make sure it runs, more or less.
	assert!(
		Command::new(&bin)
			.current_dir(&repo)
			.arg("-h")
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.map_or(false, |x| String::from_utf8_lossy(&x.stdout).contains("ZopfliPNG")),
		"Unable to run built Zopflipng executable."
	);
}

/// # In path.
///
/// This generates a (file/dir) path relative to `MANIFEST_DIR`.
fn in_path(name: &str) -> PathBuf {
	let dir = std::env::var("CARGO_MANIFEST_DIR").expect("Missing CARGO_MANIFEST_DIR.");
	std::fs::canonicalize(dir)
		.expect("Missing CARGO_MANIFEST_DIR.")
		.join(name)
}

/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	std::fs::canonicalize(dir)
		.expect("Missing OUT_DIR.")
		.join(name)
}
