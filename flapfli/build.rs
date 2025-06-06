/*!
# Flapfli: Build Script.
*/

use std::{
	fmt,
	fs::File,
	io::Write,
	ops::Range,
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

/// # Distance Extra Byts (by Symbol).
const DISTANCE_BITS: [u8; 32] = [
	0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6,
	7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13, 13, 0, 0,
];

/// # Length Symbol Bits (by Litlen).
const LENGTH_SYMBOL_BITS: [u8; 259] = [
	0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1,
	2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 3, 3, 3, 3, 3, 3,
	3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
	4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
	4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
	4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 5, 5, 5, 5, 5, 5, 5, 5,
	5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
	5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
	5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
	5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5,
	5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 0,
];

const ZOPFLI_WINDOW_SIZE: u16 = 32_768;



/// # Build.
fn main() {
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

	// Build lodepng first.
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

	let mut out = format!(
		"{}{}{}{}{}{}{}",
		NumEnum::new(0..19_u8, "Extended Deflate Indices.", "DeflateSym")
			.with_debug()
			.with_eq()
			.with_iter(),
		NumEnum::new(0..16_u8, "Basic Deflate Indices.", "DeflateSymBasic").with_eq(),
		NumEnum::new(0..32_u16, "Distance Symbols.", "Dsym"),
		NumEnum::new(0..259_u16, "Lit/Lengths.", "LitLen").with_eq().with_iter(),
		NumEnum::new(0..286_u16, "Lit/Length Symbols.", "Lsym"),
		NumEnum::new(0..15_u8, "Block Split Length.", "SplitLen").with_eq(),
		NumEnum::new(0..30_u8, "Tree Symbol Distances.", "TreeDist").with_eq(),
	);

	out.push_str(r"/// # Distance Symbols by Distance
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
		if i.is_multiple_of(128) { out.push('\n'); }
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
		if i.is_multiple_of(128) { out.push('\n'); }
		write!(&mut out, "{dvalue}, ").unwrap();
	}
	out.push_str("\n];\n");

	/// # Distance and length bits.
	///
	/// Generate integer and float constants for our bit arrays.
	fn bits_and_bobs<const N: usize>(title: &str, name: &str, arr: [u8; N]) -> String {
		format!(
			"/// # {title}.
pub(crate) const {name}: [u8; {N}] = {arr:?};

/// # {title} (Float).
///
/// This is identical to the `u8` version, but avoids a lot of `f64::from` calls.
pub(crate) const {name}_F: [f64; {N}] = {:?};
",
			arr.map(f64::from),
		)
	}

	out.push_str(&bits_and_bobs("Distance Bits (by Symbol)", "DISTANCE_BITS", DISTANCE_BITS));
	out.push_str(&bits_and_bobs("Length Bits (by Symbol)", "LENGTH_SYMBOL_BITS", LENGTH_SYMBOL_BITS));

	// Save it!
	write(&out_path("symbols.rs"), out.as_bytes());
}

/// # FFI Bindings.
fn bindings(lodepng_src: &Path) {
	bindgen::Builder::default()
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
		.expect("Unable to generate bindings")
		.write_to_file(out_path("lodepng-bindgen.rs"))
		.expect("Unable to save bindings");
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



/// # Number Enum.
///
/// We have a lot of custom numeric types that cover a range of numbers; this
/// struct ensures we generate their code consistently.
struct NumEnum<T: Copy + fmt::Display>
where Range<T>: ExactSizeIterator<Item=T> {
	rng: Range<T>,
	title: &'static str,
	name: &'static str,
	flags: u8,
}

impl<T: Copy + fmt::Display> NumEnum<T>
where Range<T>: ExactSizeIterator<Item=T> {
	const DERIVE_DEBUG: u8 = 0b0000_0001;
	const DERIVE_EQ: u8 =    0b0000_0010;
	const DERIVE_ITER: u8 =  0b0000_0100;

	/// # New Instance.
	const fn new(rng: Range<T>, title: &'static str, name: &'static str) -> Self {
		Self { rng, title, name, flags: 0 }
	}

	/// # With Derive `Debug`.
	const fn with_debug(self) -> Self {
		Self {
			flags: self.flags | Self::DERIVE_DEBUG,
			..self
		}
	}

	/// # With Derive `Eq`/`PartialEq`.
	const fn with_eq(self) -> Self {
		Self {
			flags: self.flags | Self::DERIVE_EQ,
			..self
		}
	}

	/// # With Iterator.
	const fn with_iter(self) -> Self {
		Self {
			flags: self.flags | Self::DERIVE_ITER,
			..self
		}
	}
}

impl<T: Copy + fmt::Display> fmt::Display for NumEnum<T>
where Range<T>: ExactSizeIterator<Item=T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Allow dead code.
		writeln!(f, "#[allow(dead_code, clippy::allow_attributes, clippy::missing_docs_in_private_items, reason = \"Code is auto-generated.\")]")?;

		// Representation.
		let kind = std::any::type_name::<T>();
		writeln!(f, "#[repr({kind})]")?;

		// Derives.
		write!(f, "#[derive(Clone, Copy")?;
		if Self::DERIVE_DEBUG == self.flags & Self::DERIVE_DEBUG { write!(f, ", Debug")?; }
		if Self::DERIVE_EQ == self.flags & Self::DERIVE_EQ { write!(f, ", Eq, PartialEq")?; }
		writeln!(f, ")]")?;

		// Title.
		writeln!(f, "/// # {}", self.title)?;

		// Opening.
		writeln!(f, "pub(crate) enum {} {{", self.name)?;

		// Arms.
		let width: usize = self.rng.end.to_string().len();
		let prefix: String = self.name[..1].to_ascii_uppercase();
		for i in self.rng.clone() {
			writeln!(f, "\t{prefix}{i:0width$} = {i}_{kind},", width=width)?;
		}

		// Closing.
		writeln!(f, "}}\n")?;

		let max = self.rng.clone().last().expect("Range failed.");

		// MIN, MAX, increment, decrement.
		writeln!(
			f,
			"impl {name} {{
	#[allow(dead_code, clippy::allow_attributes, reason = \"Code is auto-generated.\")]
	/// # Minimum Value.
	pub(crate) const MIN: Self = Self::{prefix}{start:0width$};

	#[allow(dead_code, clippy::allow_attributes, reason = \"Code is auto-generated.\")]
	/// # Maximum Value.
	pub(crate) const MAX: Self = Self::{prefix}{max:0width$};

	#[allow(unsafe_code, clippy::allow_attributes, dead_code, reason = \"Code is auto-generated.\")]
	/// # Decrement.
	///
	/// Return `self - 1` or None.
	pub(crate) const fn decrement(self) -> Option<Self> {{
		let n = self as {kind};
		if n == {start} {{ None }}
		else {{
			// Safety: we aren't at the bottom yet!
			Some(unsafe {{ std::mem::transmute::<{kind}, Self>(n - 1) }})
		}}
	}}

	#[allow(unsafe_code, clippy::allow_attributes, dead_code, reason = \"Code is auto-generated.\")]
	/// # Increment.
	///
	/// Return `self + 1` or None.
	pub(crate) const fn increment(self) -> Option<Self> {{
		let n = self as {kind} + 1;
		if n < {end} {{
			// Safety: we aren't at the top yet!
			Some(unsafe {{ std::mem::transmute::<{kind}, Self>(n) }})
		}}
		else {{ None }}
	}}
}}\n",
			name=self.name,
			start=self.rng.start,
			end=self.rng.end,
			width=width
		)?;

		// Symbol iterator?
		if Self::DERIVE_ITER == self.flags & Self::DERIVE_ITER {
			writeln!(
				f,
				"/// # `{name}` Iterator.
pub(crate) struct {name}Iter({kind});

impl Iterator for {name}Iter {{
	type Item = {name};

	fn next(&mut self) -> Option<Self::Item> {{
		let old = self.0;
		if old < {end} {{
			self.0 += 1;
			#[expect(unsafe_code, reason = \"For transmute.\")]
			// Safety: {kind} has the same size and alignment as {name}.
			Some(unsafe {{ std::mem::transmute::<{kind}, {name}>(old) }})
		}}
		else {{ None }}
	}}

	fn size_hint(&self) -> (usize, Option<usize>) {{
		let len = self.len();
		(len, Some(len))
	}}
}}

impl ExactSizeIterator for {name}Iter {{
	fn len(&self) -> usize {{
		usize::from({end}_{kind}.saturating_sub(self.0))
	}}
}}
",
				name=self.name,
				end=self.rng.end,
			)?;
		}

		Ok(())
	}
}
