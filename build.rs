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
	println!("cargo:rerun-if-changed=./skel/vendor/");

	#[cfg(not(target_pointer_width = "64"))]
	panic!("Flaca requires a 64-bit CPU architecture.");

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

	write(&out_path("flaca-extensions.rs"), out.as_bytes());
}

/// # Build `zopfli`/`lodepng`.
///
/// The Rust ports of these libraries are missing features that noticeably
/// affect PNG compression, and are quite a bit slower than the original C
/// libraries as well. Unless/until that changes, we'll have to work with the
/// originals.
///
/// The relevant `zopflipng` bits, though, were easily ported to the Flaca
/// library proper, so we can at least avoid the headaches associated with C++
/// interop!
fn build_ffi() {
	// Define some paths.
	let repo = Path::new("skel/vendor");
	let zopfli_src = repo.join("zopfli");
	let lodepng_src = repo.join("lodepng");

	// Build Zopfli first.
	let mut c = cc::Build::new();
	c.includes(&[repo, &lodepng_src, &zopfli_src])
		.cpp(false)
		.flag_if_supported("-W")
		.flag_if_supported("-ansi")
		.flag_if_supported("-pedantic")
		.flag_if_supported("-Wlm")
		.pic(true)
		.static_flag(true)
		.files(&[
			zopfli_src.join("zopfli.c"),
			lodepng_src.join("lodepng.c"),
		])
		.define("LODEPNG_NO_COMPILE_DISK", None)
		.define("LODEPNG_NO_COMPILE_CPP", None)
		.compile("zopflipng");

	bindings(repo, &lodepng_src, &zopfli_src);
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

/// # FFI Bindings.
///
/// These have been manually transcribed into the Rust sources, but this
/// commented-out code can be re-enabled if they ever need to be updated.
fn bindings(repo: &Path, lodepng_src: &Path, zopfli_src: &Path) {
	let bindings = bindgen::Builder::default()
		.header(lodepng_src.join("lodepng.h").to_string_lossy())
		.header(repo.join("rust.h").to_string_lossy())
		.header(zopfli_src.join("zopfli.h").to_string_lossy())
		.allowlist_function("lodepng_color_mode_copy")
		.allowlist_function("lodepng_color_stats_init")
		.allowlist_function("lodepng_compute_color_stats")
		.allowlist_function("lodepng_decode")
		.allowlist_function("lodepng_encode")
		.allowlist_function("lodepng_state_cleanup")
		.allowlist_function("lodepng_state_init")
		.allowlist_function("ZopfliAddBit")
		.allowlist_function("ZopfliAddBits")
		.allowlist_function("ZopfliAddHuffmanBits")
		.allowlist_function("ZopfliAddNonCompressedBlock")
		.allowlist_function("ZopfliAppendLZ77Store")
		.allowlist_function("ZopfliCleanLZ77Store")
		.allowlist_function("ZopfliCopyLZ77Store")
		.allowlist_function("ZopfliEncodeTree")
		.allowlist_function("ZopfliInitLZ77Store")
		.allowlist_function("ZopfliStoreLitLenDist")
		.allowlist_type("LodePNGColorStats")
		.allowlist_type("LodePNGCompressSettings")
		.allowlist_type("LodePNGState")
		.allowlist_type("ZopfliLZ77Store")
		.rustified_enum("LodePNGColorType")
		.rustified_enum("LodePNGFilterStrategy")
		.derive_debug(true)
		.merge_extern_blocks(true)
		.no_copy("LodePNGState")
		.size_t_is_usize(true)
		.sort_semantically(true)
		.generate()
		.expect("Unable to generate bindings");

	// Save the bindings to a string.
	let mut out = Vec::new();
	bindings.write(Box::new(&mut out)).expect("Unable to write bindings.");
	let mut out = String::from_utf8(out)
		.expect("Bindings contain invalid UTF-8.")
		.replace("    ", "\t");

	// Move the tests out into their own string so we can include them in a
	// test-specific module.
	let mut tests = String::new();
	while let Some(from) = out.find("#[test]") {
		let sub = &out[from..];
		let Some(to) = sub.find("\n}\n") else { break; };
		let test = &sub[..to + 3];
		assert!(
			test.starts_with("#[test]") && test.ends_with("\n}\n"),
			"Invalid binding test clip:\n{test:?}\n",
		);

		tests.push_str(test);
		out.replace_range(from..from + to + 3, "");
	}

	// Allow dead code for these two enum variants we aren't using.
	for i in ["LCT_MAX_OCTET_VALUE = 255,", "LFS_PREDEFINED = 8,"] {
		out = out.replace(i, &format!("#[allow(dead_code)] {i}"));
	}

	// Switch from pub to pub(super).
	out = out.replace("pub ", "pub(super) ");

	// Double-check our replacements were actually for visibility, rather than
	// an (unlikely) accidental substring match like "mypub = 5". That would
	// generate a compiler error on its own, but this makes it clearer what
	// went wrong.
	for w in out.as_bytes().windows(12) {
		if w.ends_with(b"pub(super) ") {
			assert!(
				w[0].is_ascii_whitespace(),
				"Invalid bindgen visibility replacement!",
			);
		}
	}

	// Write the bindings and tests.
	write(&out_path("lodepng-bindgen.rs"), out.as_bytes());
	write(&out_path("lodepng-bindgen-tests.rs"), tests.as_bytes());
}
