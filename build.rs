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

	// Git clone.
	let repo = out_path("zopfli-git");
	if ! repo.is_dir() && ! Command::new("git")
			.args(&[
				OsStr::new("clone"),
				OsStr::new("https://github.com/google/zopfli"),
				repo.as_os_str(),
			])
			.stdout(Stdio::null())
			.stderr(Stdio::null())
			.status()
			.map_or(false, |s| s.success()) {
		panic!("Unable to clone Zopfli repo.");
	}

	// Build it.
	if ! Command::new("make")
		.current_dir(&repo)
		.args(&["zopflipng"])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status()
		.map_or(false, |s| s.success()) {
		panic!("Unable to build Zopflipng.");
	}

	// The bin should exist now.
	let bin = out_path("zopfli-git/zopflipng");
	assert!(bin.is_file(), "Missing built Zopflipng executable.");

	// Strip it, but if this fails that's all right.
	let _res = Command::new("strip")
		.args(&[bin.as_os_str()])
		.stdout(Stdio::null())
		.stderr(Stdio::null())
		.status();
}

/// # Out path.
///
/// This is a workaround for `cargo-deb` not being able to find files in the
/// `OUT_DIR`. Adapted from [this example](https://github.com/mmstick/cargo-deb/blob/e43018a46b8dc922cfdf6cdde12f7ed92fcc41aa/example/build.rs).
fn out_path(name: &str) -> PathBuf {
	let out = PathBuf::from(std::env::var("OUT_DIR").expect("Missing OUT_DIR."));
	let mut out = out
		.ancestors()  // .../target/x86_64-unknown-linux-gnu/<debug|release>/build/flaca-<SHA>/out
		.nth(3)       // .../target/x86_64-unknown-linux-gnu/<debug|release>
		.expect("Missing OUT_DIR.")
		.to_owned();

	out.push(name);
	out
}
