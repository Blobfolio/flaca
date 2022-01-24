/*!
# Flaca - Build
*/

use std::{
	ffi::OsStr,
	fs::File,
	io::Write,
	path::PathBuf,
	process::{
		Command,
		Stdio,
	},
};



macro_rules! cmd {
	($cmd:expr, $oops:literal) => (
		assert!(
			$cmd
				.stdout(Stdio::null())
				.stderr(Stdio::null())
				.status()
				.map_or(false, |s| s.success()),
			$oops
		);
	);
}



/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to link
/// to the static libs for some FFI action instead.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	// This is where Zopflipng's files will go.
	let repo = out_path("zopfli-git");

	// If the folder already exists, nuke it.
	if repo.is_dir() {
		std::fs::remove_dir_all(&repo).expect("Unable to clear old repo.");
	}

	// Clone the repository.
	cmd!(
		Command::new("git")
			.args(&[
				OsStr::new("clone"),
				OsStr::new("https://github.com/google/zopfli"),
				repo.as_os_str(),
			]),
		"Unable to clone Zopfli git repository."
	);

	// Checkout a specific commit for reproducibility.
	cmd!(
		Command::new("git").current_dir(&repo).args(&["checkout", "831773b"]),
		"Unable to checkout Zopfli git repository."
	);

	{
		// Patch the Makefile to enable LTO.
		let makefile = repo.join("Makefile");
		let content = std::fs::read_to_string(&makefile)
			.expect("Missing Makefile.")
			.replace("-O3", "-O3 -flto");
		File::create(makefile)
			.and_then(|mut file| file.write_all(content.as_bytes()).and_then(|_| file.flush()))
			.expect("Unable to patch Makefile.");
	}

	// Build it.
	cmd!(
		Command::new("make").current_dir(&repo).args(&["libzopfli.a"]),
		"Unable to build libzopfli.a."
	);
	cmd!(
		Command::new("make").current_dir(&repo).args(&["libzopflipng.a"]),
		"Unable to build libzopflipng.a."
	);

	// Link up the libraries.
	println!("cargo:rustc-link-lib=static=zopfli");
	println!("cargo:rustc-link-lib=static=zopflipng");
	println!("cargo:rustc-link-search=native={}", repo.to_string_lossy());

	// bindings();
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

/// # Out path.
///
/// This generates a (file/dir) path relative to `OUT_DIR`.
fn out_path(name: &str) -> PathBuf {
	let dir = std::env::var("OUT_DIR").expect("Missing OUT_DIR.");
	std::fs::canonicalize(dir)
		.expect("Missing OUT_DIR.")
		.join(name)
}
