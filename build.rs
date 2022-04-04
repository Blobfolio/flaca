/*!
# Flaca - Build
*/

use std::{
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

/// # Repo Revision.
///
/// For reproducibility, we'll be working from a specific commit.
const REVISION: &str = "831773b";



/// # Build Zopflipng.
///
/// Rust's Zopfli implementation is insufficient for our needs; we have to link
/// to the static libs for some FFI action instead.
pub fn main() {
	println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION");

	// This is where Zopflipng's files will go.
	let repo = out_path("zopfli-git");
	let repo_str = repo.to_str().expect("Repo path contains invalid UTF-8.");

	// Clone the repository if needed.
	if ! repo.is_dir() {
		// Clone the repository.
		cmd!(
			Command::new("git")
				.args(&[
					"clone",
					"https://github.com/google/zopfli",
					repo_str,
				]),
			"Unable to clone Zopfli git repository."
		);
	}

	// Checkout.
	cmd!(
		Command::new("git").current_dir(&repo).args(&["checkout", REVISION]),
		"Unable to checkout Zopfli git repository."
	);

	// We might need to patch the makefile to enable LTO.
	let makefile = repo.join("Makefile");
	let content = std::fs::read_to_string(&makefile).expect("Missing Makefile.");
	if ! content.contains("-O3 -flto") {
		let content = content.replace("-O3", "-O3 -flto");
		File::create(makefile)
			.and_then(|mut file| file.write_all(content.as_bytes()).and_then(|_| file.flush()))
			.expect("Unable to patch Makefile.");
	}

	// Build the main zopfli library.
	if ! repo.join("libzopfli.a").exists() {
		cmd!(
			Command::new("make").current_dir(&repo).args(&["libzopfli.a"]),
			"Unable to build libzopfli.a."
		);
	}

	println!("cargo:rustc-link-lib=static=zopfli");
	println!("cargo:rustc-link-search=native={}", repo_str);

	// Build the zopflipng library.
	if ! repo.join("libzopflipng.a").exists() {
		cmd!(
			Command::new("make").current_dir(&repo).args(&["libzopflipng.a"]),
			"Unable to build libzopflipng.a."
		);
	}

	println!("cargo:rustc-link-lib=static=zopflipng");

	// Link up C++ too, and then we're done!
	#[cfg(any(target_os = "macos", target_os = "ios", target_os = "freebsd"))]
    println!("cargo:rustc-link-lib=c++");

    #[cfg(not(any(target_os = "macos", target_os = "ios", target_os = "freebsd")))]
    println!("cargo:rustc-link-lib=stdc++");
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
