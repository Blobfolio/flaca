/*!
# Flapfli: Errors.
*/

#[cfg(debug_assertions)]
use std::fmt;



#[cfg(not(debug_assertions))]
/// # Error (Release).
///
/// This library uses `Result` return types like conditionally-panicking
/// assertions. (Error responses shouldn't actually be possible, but, well,
/// bugs happen!)
///
/// When debug assertions are _disabled_, errors are bubbled up to lodepng,
/// allowing it to gracefully abandon its efforts.
pub(crate) type ZopfliError = ();



#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Error (Debug).
///
/// When debug assertions are _enabled_, error responses panic with the
/// relevant source details to aid further investigation.
///
/// This struct stores those details, allowing us to delay the panicking until
/// the error has bubbled back to lodepng.
pub(crate) struct ZopfliError {
	/// # Source File.
	file: &'static str,

	/// # Source Line.
	line: u32,
}

#[cfg(debug_assertions)]
impl ZopfliError {
	#[cfg(debug_assertions)]
	/// # New Error.
	pub(crate) const fn new(file: &'static str, line: u32) -> Self {
		Self { file, line }
	}
}

#[cfg(debug_assertions)]
impl fmt::Display for ZopfliError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_fmt(format_args!(
			"Zopfli BUG!!! Sanity check failed at {}:{}",
			self.file,
			self.line,
		))
	}
}



#[cfg(debug_assertions)]
/// # Error Macro (Debug).
///
/// The debug version of this macro panics with a message indicating the file
/// and line number to aid further investigation.
macro_rules! zopfli_error { () => (ZopfliError::new(file!(), line!())); }

#[cfg(not(debug_assertions))]
/// # Error Macro (Release).
///
/// The non-debug version simply returns a `()`.
macro_rules! zopfli_error { () => (()); }

/// # Expose the macro to the rest of the module.
pub(super) use zopfli_error;
