/*!
# Flaca - Build
*/

/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to link
/// to the static libs for some FFI action instead.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

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
