/*!
# Flapfli: Build Script.
*/

use std::{
	fs::File,
	io::Write,
	path::{
		Path,
		PathBuf,
	},
};

/// # Distance Extra Bits Value Masks.
const DISTANCE_EXTRA_BITS_MASK: [(u32, u32); 16] = [
	(0, 0), (0, 0), (5, 1), (9, 3), (17, 7), (33, 15), (65, 31), (129, 63),
	(257, 127), (513, 255), (1025, 511), (2049, 1023), (4097, 2047),
	(8193, 4095), (16_385, 8191), (32_769, 16_383),
];

const ZOPFLI_WINDOW_SIZE: u16 = 32_768;



/// # Build.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");
	println!("cargo:rerun-if-env-changed=TARGET_CPU");
	println!("cargo:rerun-if-changed=../skel/vendor/");

	#[cfg(not(target_pointer_width = "64"))]
	panic!("Flaca requires a 64-bit CPU architecture.");

	build_ffi();
	build_symbols();
}

/// # Build `lodepng`.
///
/// The Rust port of `lodepng` is missing some functionality that is required
/// to fully emulate `zopflipng`, so we're stuck with the C version until I
/// decide to completely rewrite that too. Haha.
fn build_ffi() {
	// Define some paths.
	let repo = Path::new("../skel/vendor");
	let lodepng_src = repo.join("lodepng");

	// Build Zopfli first.
	let mut c = cc::Build::new();
	c.includes([repo, &lodepng_src])
		.cpp(false)
		.flag_if_supported("-W")
		.flag_if_supported("-ansi")
		.flag_if_supported("-pedantic")
		.pic(true)
		.static_flag(true)
		.files([
			lodepng_src.join("lodepng.c"),
		])
		.define("LODEPNG_NO_COMPILE_ALLOCATORS", None)
		.define("LODEPNG_NO_COMPILE_ANCILLARY_CHUNKS", None)
		.define("LODEPNG_NO_COMPILE_CPP", None)
		.define("LODEPNG_NO_COMPILE_CRC", None)
		.define("LODEPNG_NO_COMPILE_DISK", None)
		.compile("lodepng");

	bindings(&lodepng_src);
}

/// # Build Symbols.
///
/// The compiler struggles with Zopfli's litlen-distance-symbol-as-index
/// structures. Enums are a silly but simple way to help it better understand
/// the boundaries.
///
/// Plus they're easy to automate, like so:
fn build_symbols() {
	use std::fmt::Write;

	let mut out = r"#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Whackadoodle Deflate Indices.
pub(crate) enum DeflateSym {".to_owned();
	for i in 0..19 {
		write!(&mut out, "\n\tD{i:02} = {i}_u8,").unwrap();
	}
	out.push_str(r"
}

#[allow(dead_code)]
#[repr(u16)]
#[derive(Clone, Copy)]
/// # Distance Symbols.
pub(crate) enum Dsym {");
	for i in 0..32 {
		write!(&mut out, "\n\tD{i:02} = {i}_u16,").unwrap();
	}
	out.push_str(r"
}

#[allow(dead_code)]
#[repr(u16)]
#[derive(Clone, Copy)]
/// # Lit/Lengths.
pub(crate) enum LitLen {");
	for i in 0..259 {
		write!(&mut out, "\n\tL{i:03} = {i}_u16,").unwrap();
	}
	out.push_str(r"
}

#[allow(dead_code)]
#[repr(u16)]
#[derive(Clone, Copy)]
/// # Lit/Len Symbols.
pub(crate) enum Lsym {");
	for i in 0..=285 {
		write!(&mut out, "\n\tL{i:03} = {i}_u16,").unwrap();
	}
	out.push_str("
}

/// # Distance Symbols by Distance
///
/// This table is kinda terrible, but the performance gains (versus calculating
/// the symbols on-the-fly) are incredible, so whatever.
pub(crate) const DISTANCE_SYMBOLS: &[Dsym; 32_768] = &[");
	// Apologies… this might be somewhat slow to build, but better now than at
	// runtime!
	for i in 0..ZOPFLI_WINDOW_SIZE {
		let dsym =
			if i < 5 { i.saturating_sub(1) }
			else {
				let d_log = (i - 1).ilog2();
				let r = ((i as u32 - 1) >> (d_log - 1)) & 1;
				(d_log * 2 + r) as u16
			};

		// Add some line breaks, but not too many!
		if i % 128 == 0 { out.push('\n'); }
		write!(&mut out, "Dsym::D{dsym:02}, ").unwrap();
	}
	out.push_str("
];

/// # Distance Bit Values by Distance.
///
/// Same as the symbol table, but for an obscure value used in only one
/// hot-hot place. Haha.
pub(crate) const DISTANCE_VALUES: &[u16; 32_768] = &[");
	for i in 0..ZOPFLI_WINDOW_SIZE {
		let dvalue =
			if i < 5 { 0 }
			else {
				let d_log = (i - 1).ilog2();
				let (m1, m2) = DISTANCE_EXTRA_BITS_MASK[d_log as usize];
				(i as u32 - m1) & m2
			};

		// Add some line breaks, but not too many!
		if i % 128 == 0 { out.push('\n'); }
		write!(&mut out, "{dvalue}, ").unwrap();
	}
	out.push_str("\n];\n");

	// Save it!
	write(&out_path("symbols.rs"), out.as_bytes());
}

/// # FFI Bindings.
///
/// These have been manually transcribed into the Rust sources, but this
/// commented-out code can be re-enabled if they ever need to be updated.
fn bindings(lodepng_src: &Path) {
	let bindings = bindgen::Builder::default()
		.clang_args([
			"-DLODEPNG_NO_COMPILE_ALLOCATORS",
			"-DLODEPNG_NO_COMPILE_ANCILLARY_CHUNKS",
			"-DLODEPNG_NO_COMPILE_CPP",
			"-DLODEPNG_NO_COMPILE_CRC",
			"-DLODEPNG_NO_COMPILE_DISK",
		])
		.header(lodepng_src.join("lodepng.h").to_string_lossy())
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
	for i in ["pub enum LodePNGFilterStrategy", "pub enum LodePNGColorType"] {
		out = out.replace(i, &format!("#[allow(dead_code)]\n{i}"));
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
