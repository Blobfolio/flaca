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
	println!("cargo:rerun-if-changed=./skel/vendor/");

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
		.includes(&[repo, &lodepng_src, &zopfli_src])
		.cpp(false)
		.flag_if_supported("-W")
		.flag_if_supported("-ansi")
		.flag_if_supported("-pedantic")
		.flag_if_supported("-lm")
		.flag_if_supported("-Wno-unused-function")
		.flag_if_supported("-Wno-unused")
		.pic(true)
		.static_flag(true)
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
		.define("LODEPNG_NO_COMPILE_DISK", None)
		.define("LODEPNG_NO_COMPILE_CPP", None)
		.compile("zopflipng");

	// bindings(repo, &lodepng_src);
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}

/*
/// # FFI Bindings.
///
/// These have been manually transcribed into the Rust sources, but this
/// commented-out code can be re-enabled if they ever need to be udpated.
fn bindings(repo: &Path, lodepng_src: &Path) {
	let bindings = bindgen::Builder::default()
		.header(lodepng_src.join("lodepng.h").to_string_lossy())
		.header(repo.join("custom_png_deflate.h").to_string_lossy())
		.allowlist_function("custom_png_deflate")
		.allowlist_function("lodepng_color_mode_copy")
		.allowlist_function("lodepng_color_stats_init")
		.allowlist_function("lodepng_compute_color_stats")
		.allowlist_function("lodepng_decode")
		.allowlist_function("lodepng_encode")
		.allowlist_function("lodepng_state_cleanup")
		.allowlist_function("lodepng_state_init")
		.allowlist_type("LodePNGColorStats")
		.allowlist_type("LodePNGCompressSettings")
		.allowlist_type("LodePNGState")
		.size_t_is_usize(true)
		.rustified_enum("LodePNGColorType")
		.rustified_enum("LodePNGFilterStrategy")
		.derive_debug(true)
		.generate()
		.expect("Unable to generate bindings");

	let out_path = std::fs::canonicalize(std::env::var("OUT_DIR").expect("Missing OUT_DIR."))
		.expect("Missing OUT_DIR.")
		.join("flaca-bindings.rs");

	bindings
		.write_to_file(&out_path)
		.expect("Couldn't write bindings!");
}
*/
