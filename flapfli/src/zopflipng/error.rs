/*!
# Flapfli: Errors.
*/

use std::fmt;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Zopfli Error.
///
/// This struct is used for logical failings (bugs) in the ported zopfli
/// functionality. This shouldn't ever be instantiated in practice…
///
/// When compiled with `debug-assertions = true`, an error will panic with the
/// offending source file and line number details to aid investigation.
///
/// Otherwise it simply serves as a flag for lodepng, letting it know that
/// something went wrong so it can abandon its compressive efforts for the
/// given image.
///
/// The macro `zopfli_error!` is used internally to populate the appropriate
/// details or not.
pub(crate) struct ZopfliError {
	#[cfg(debug_assertions)] file: &'static str,
	#[cfg(debug_assertions)] line: u32,
}

impl ZopfliError {
	#[cfg(debug_assertions)]
	/// # New Error.
	pub(crate) const fn new(file: &'static str, line: u32) -> Self {
		Self { file, line }
	}
}

impl fmt::Display for ZopfliError {
	#[cfg(debug_assertions)]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_fmt(format_args!(
			"Zopfli BUG!!! Sanity check failed at {}:{}",
			self.file,
			self.line,
		))
	}

	#[cfg(not(debug_assertions))]
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str("zopfli bug")
	}
}

impl std::error::Error for ZopfliError {}



#[cfg(debug_assertions)]
/// # Error Macro.
///
/// Initialize a new error with the appropriate environmental argument(s)
/// according to `debug-assertions`.
macro_rules! zopfli_error {
	() => (ZopfliError::new(file!(), line!()));
}

#[cfg(not(debug_assertions))]
/// # Error Macro.
///
/// Initialize a new error with the appropriate environmental argument(s)
/// according to `debug-assertions`.
macro_rules! zopfli_error {
	() => (ZopfliError {});
}

/// # Expose it to the rest of the module.
pub(super) use zopfli_error;
