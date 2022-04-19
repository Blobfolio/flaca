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
	build_zopfli();
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
fn build_zopfli() {
	// Define some paths.
	let repo = std::fs::canonicalize("vendor").expect("Missing vendor directory.");
	let zopfli_src = repo.join("src/zopfli");
	let zopflipng_src = repo.join("src/zopflipng");
	let lodepng_src = repo.join("src/zopflipng/lodepng");

	// Easy abort.
	if ! zopfli_src.is_dir() {
		panic!("Missing zopfli sources; you might need to initialize the ./vendor submodule.");
	}

	// Build the C first.
	cc::Build::new()
		.include(&zopfli_src)
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
		])
		.compile("zopfli");

	// And now the C++.
	cc::Build::new()
		.includes(&[
			&lodepng_src,
			&zopflipng_src,
		])
		.flag_if_supported("-ansi")
		.flag_if_supported("-pedantic")
		.opt_level(3)
		.pic(true)
		.static_flag(true)
		.warnings(false)
		.cpp(true)
		.files(&[
			lodepng_src.join("lodepng.cpp"),
			lodepng_src.join("lodepng_util.cpp"),
			zopflipng_src.join("zopflipng_lib.cc"),
		])
		.compile("zopflipng");
}

/// # Write File.
fn write(path: &Path, data: &[u8]) {
	File::create(path).and_then(|mut f| f.write_all(data).and_then(|_| f.flush()))
		.expect("Unable to write file.");
}

/*
/// # Bindings.
///
/// These have been manually transcribed and brought into the project source,
/// but for reference, this code was used to generate a rough blueprint for us.
fn bindings() {
	let root_zopfli = out_path("zopfli-git/src/zopfli");
	let root_zopflipng = out_path("zopfli-git/src/zopflipng");
	let root_lodepng = out_path("zopfli-git/src/zopflipng/lodepng");

	let bindings = bindgen::Builder::default()
		.clang_args(&[
			"-x",
			"c++",
			"-std=c++14",
		])
		.header(root_zopfli.join("zopfli.h").to_string_lossy())
		.header(root_zopflipng.join("zopflipng_lib.h").to_string_lossy())
		.header(root_lodepng.join("lodepng.h").to_string_lossy())
		.header(root_lodepng.join("lodepng_util.h").to_string_lossy())
		.allowlist_type("ZopfliPNGFilterStrategy")
		.allowlist_type("CZopfliPNGOptions")
		.allowlist_function("CZopfliPNGOptimize")
		.derive_debug(true)
		.generate()
		.expect("Unable to generate bindings");

	bindings
		.write_to_file(out_path("bindings.rs"))
		.expect("Couldn't write bindings!");
}
*/
