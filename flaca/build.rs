/*!
# Flaca - Build
*/

use argyle::FlagsBuilder;
use std::path::PathBuf;



/// # Build.
fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-env-changed=TARGET_CPU");
	println!("cargo:rerun-if-changed=../skel/vendor/");

	#[cfg(not(target_pointer_width = "64"))]
	panic!("Flaca requires a 64-bit CPU architecture.");

	build_kinds();
}

/// # Build Image Kinds.
fn build_kinds() {
	FlagsBuilder::new("ImageKind")
		.with_docs("# Image Kind.")
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
