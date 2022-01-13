/*!
# Flaca - Build
*/

/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to link
/// to the static libs for some FFI action instead.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-changed=skel/inc/libzopfli.a");
	println!("cargo:rerun-if-changed=skel/inc/libzopflipng.a");

	println!("cargo:rustc-link-lib=static=zopfli");
	println!("cargo:rustc-link-lib=static=zopflipng");
	println!("cargo:rustc-link-search=native=./skel/inc");

	/*
	// The bindings have been manually transcribed into the project source, but
	// for reference, this is how they were originally generated.
	let bindings = bindgen::Builder::default()
		.clang_args(&[
			"-x",
			"c++",
			"-std=c++14",
		])
		.header("./skel/inc/zopfli/zopfli.h")
		.header("./skel/inc/zopflipng/zopflipng_lib.h")
		.header("./skel/inc/zopflipng/lodepng/lodepng.h")
		.header("./skel/inc/zopflipng/lodepng/lodepng_util.h")
		.allowlist_type("ZopfliPNGFilterStrategy")
		.allowlist_type("CZopfliPNGOptions")
		.allowlist_function("CZopfliPNGOptimize")
		.derive_debug(true)
		.generate()
		.expect("Unable to generate bindings");

	bindings
		.write_to_file(out_path("bindings.rs"))
		.expect("Couldn't write bindings!");*/
}


/*
/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> std::path::PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	std::fs::canonicalize(dir)
		.expect("Missing OUT_DIR.")
		.join(name)
}
*/
