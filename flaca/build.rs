/*!
# Flaca - Build
*/

use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};



/// # Build.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-env-changed=TARGET_CPU");
	println!("cargo:rerun-if-changed=../skel/vendor/");

	#[cfg(not(target_pointer_width = "64"))]
	panic!("Flaca requires a 64-bit CPU architecture.");

	build_exts();
}

/// # Pre-Compute Extensions.
///
/// We might as well generate the path-matching constants while we're here.
fn build_exts() {
	let out = format!(
		r"
/// # Extension: JPEG.
const E_JPEG: Extension = {};

/// # Extension: JPG.
const E_JPG: Extension = {};

/// # Extension: PNG.
const E_PNG: Extension = {};
",
		Extension::codegen(b"jpeg"),
		Extension::codegen(b"jpg"),
		Extension::codegen(b"png"),
	);

	write(&out_path("flaca-extensions.rs"), out.as_bytes());
}

/// # Output Path.
///
/// Append the sub-path to OUT_DIR and return it.
fn out_path(stub: &str) -> PathBuf {
	std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join(stub)
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}
