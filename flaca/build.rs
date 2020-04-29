/*!
# Build
*/

extern crate clap;

include!("src/menu.rs");



/// Build tasks.
fn main() {
	completions();
}

/// Bash completions.
fn completions() {
	use clap::Shell;
	use std::path::PathBuf;

	// Store the completions here.
	let outdir: PathBuf = PathBuf::from("../release/completions");
	if ! outdir.is_dir() {
		std::fs::create_dir(&outdir).expect("Unable to create temporary completion directory.");
	}

	// Complete it!
	menu().gen_completions(
		"flaca",
		Shell::Bash,
		outdir
	);
}
