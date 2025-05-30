/*!
# Flaca - Build
*/

use argyle::{
	FlagsBuilder,
	KeyWordsBuilder,
};
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

	build_cli();
	build_exts();
	build_kinds();
}

/// # Build CLI Arguments.
fn build_cli() {
	let mut builder = KeyWordsBuilder::default();
	builder.push_keys([
		"-h", "--help",
		"--no-gif",
		"--no-jpg", "--no-jpeg",
		"--no-png",
		"--no-symlinks",
		"--preserve-times",
		"-p", "--progress",
		"-V", "--version",
	]);
	builder.push_keys_with_values([
		"-j",
		"-l", "--list",
		"--max-pixels",
		"-z",
	]);
	builder.save(out_path("argyle.rs"));
}

/// # Pre-Compute Extensions.
///
/// We might as well generate the path-matching constants while we're here.
fn build_exts() {
	let out = format!(
		r"
/// # Extension: GIF.
const E_GIF: Extension = {};

/// # Extension: JPEG.
const E_JPEG: Extension = {};

/// # Extension: JPG.
const E_JPG: Extension = {};

/// # Extension: PNG.
const E_PNG: Extension = {};
",
		Extension::codegen(b"gif"),
		Extension::codegen(b"jpeg"),
		Extension::codegen(b"jpg"),
		Extension::codegen(b"png"),
	);

	write(&out_path("flaca-extensions.rs"), out.as_bytes());
}

/// # Build Image Kinds.
fn build_kinds() {
	FlagsBuilder::new("ImageKind")
		.with_flag("Gif", Some("# GIF."))
		.with_flag("Jpeg", Some("# JPEG."))
		.with_flag("Png", Some("# PNG."))
		.with_alias("All", ["Gif", "Jpeg", "Png"], Some("# All Three Kinds."))
		.save(out_path("flaca-kinds.rs"));
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
