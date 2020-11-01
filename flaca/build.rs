#[cfg(feature = "man")]
/// # Build BASH Completions.
///
/// We can do this in the same run we use for building the MAN pages.
fn main() {
	use fyi_menu::Basher;
	use std::{
		env,
		path::PathBuf,
	};

	// We're going to shove this in "flaca/misc/flaca.bash". If we used
	// `OUT_DIR` like Cargo suggests, we'd never be able to find it to shove
	// it into the `.deb` package.
	let mut path: PathBuf = env::var("CARGO_MANIFEST_DIR")
		.ok()
		.and_then(|x| std::fs::canonicalize(x).ok())
		.map(|x|
			x.parent()
				.expect("Missing completion script directory.")
				.to_path_buf()
		)
		.expect("Missing completion script directory.");

	path.push("skel/completions");
	path.push("flaca.bash");

	// All of our options.
	let b = Basher::new("flaca")
		.with_option(Some("-l"), Some("--list"))
		.with_switch(None, Some("--clean"))
		.with_switch(Some("-h"), Some("--help"))
		.with_switch(Some("-p"), Some("--progress"))
		.with_switch(Some("-V"), Some("--version"));

	// Write it!
	b.write(&path)
		.unwrap_or_else(|_| panic!("Unable to write completion script: {:?}", path));
}

#[cfg(not(feature = "man"))]
/// # Do Nothing.
fn main() {}
