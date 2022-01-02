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
	assert!(
		Command::new("git")
			.args(&[
				OsStr::new("clone"),
				OsStr::new("https://github.com/google/zopfli"),
				repo.as_os_str(),
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_or(false, |s| s.success()),
		"Unable to clone Zopfli repo."
	);

	// Local patch path.
	let patch = std::fs::canonicalize(
		PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")
			.expect("Missing Manifest Dir"))
			.join("skel/zopfli.patch")
	).expect("Missing Zopfli patch.");

	// Apply the patch so we can build the binary with LTO.
	assert!(
		Command::new("git")
			.current_dir(&repo)
			.args(&[
				OsStr::new("apply"),
				patch.as_os_str(),
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_or(false, |s| s.success()),
		"Unable to patch Zopfli."
	);

	// Build it.
	assert!(
		Command::new("make")
		.current_dir(&repo)
		.args(&["zopflipng"])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_or(false, |s| s.success()),
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
			.args(&["-h"])
			.stdout(Stdio::piped())
			.stderr(Stdio::piped())
			.output()
			.map_or(false, |x| String::from_utf8_lossy(&x.stdout).contains("ZopfliPNG")),
		"Unable to run built Zopflipng executable."
	);
}

/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	let mut out = std::fs::canonicalize(dir).expect("Missing OUT_DIR.");
	out.push(name);
	out
}
