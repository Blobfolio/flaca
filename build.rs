/*!
# Flaca - Build
*/

use dowser::Extension;
use std::{
	fs::File,
	io::Write,
	path::Path,
};



/// # Build.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	build_exts();
	build_ffi();
}

/// # Pre-Compute Extensions.
///
/// We might as well generate the path-matching constants while we're here.
fn build_exts() {
	let out = format!(
		r"
const E_JPEG: Extension = {};
const E_JPG: Extension = {};
const E_PNG: Extension = {};
",
		Extension::codegen(b"jpeg"),
		Extension::codegen(b"jpg"),
		Extension::codegen(b"png"),
	);

	let out_path = std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join("flaca-extensions.rs");

	write(&out_path, out.as_bytes());
}

/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to link
/// to the static libs for some FFI action instead.
fn build_ffi() {
	// Define some paths.
	let repo = Path::new("skel/vendor");
	let zopfli_src = repo.join("zopfli");
	let lodepng_src = repo.join("lodepng");

	// Build Zopfli first.
	cc::Build::new()
		.include(&zopfli_src)
		.include(&lodepng_src)
		.flag_if_supported("-ansi")
		.flag_if_supported("-pedantic")
		.opt_level(3)
		.pic(true)
		.static_flag(true)
		.warnings(false)
		.files(&[
			zopfli_src.join("blocksplitter.c"),
			zopfli_src.join("cache.c"),
			zopfli_src.join("deflate.c"),
			zopfli_src.join("hash.c"),
			zopfli_src.join("katajainen.c"),
			zopfli_src.join("lz77.c"),
			zopfli_src.join("squeeze.c"),
			zopfli_src.join("tree.c"),
			zopfli_src.join("util.c"),
			lodepng_src.join("lodepng.c"),
			repo.join("custom_png_deflate.c"),
		])
		.compile("zopfli");

	// bindings_lodepng(&lodepng_src);
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}

/*
fn bindings_lodepng(lodepng_src: &Path) {
	let bindings = bindgen::Builder::default()
		.header(lodepng_src.join("lodepng.h").to_string_lossy())
		.allowlist_function("lodepng_decode")
		.allowlist_function("lodepng_encode")
		.allowlist_function("lodepng_color_mode_copy")
		.allowlist_function("lodepng_color_stats_init")
		.allowlist_function("lodepng_compute_color_stats")
		.allowlist_function("lodepng_state_cleanup")
		.allowlist_function("lodepng_state_init")
		.allowlist_function("custom_png_deflate")
		.allowlist_type("LodePNGColorStats")
		.allowlist_type("LodePNGCompressSettings")
		.allowlist_type("LodePNGState")
		.derive_debug(true)
		.generate()
		.expect("Unable to generate bindings");

	let out_path = std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join("lodepng-bindings.rs");

	bindings
		.write_to_file(&out_path)
		.expect("Couldn't write bindings!");
}
*/
