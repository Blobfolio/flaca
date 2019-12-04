/*!
# Error

Because _every_ Rust crate needs its very own interpretation of an error
response!
*/

use std::fmt;



#[derive(Debug, Clone, Eq, PartialEq)]
/// An Error!
pub enum Error {
	/// Flaca is already running.
	DoubleRun,
	/// The image app is invalid.
	InvalidApp,
	/// Invalid path.
	InvalidPath(String),
	/// File copy failed.
	IOCopy(String, String),
	/// File delete failed.
	IODelete(String),
	/// File move failed.
	IOMove(String, String),
	/// Missing app.
	NoApp(String),
	/// Missing apps.
	NoApps(String),
	/// No images.
	NoImages,
	/// Not a JPEG.
	NotJpeg(String),
	/// Not a PNG.
	NotPng(String),
	/// Null Log.
	NullAlert,
	/// A pass-through error.
	Other(String),
}

impl std::error::Error for Error {
	/// Description.
	fn description(&self) -> &str {
		""
	}
}

impl fmt::Display for Error {
	#[inline]
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.as_string())
	}
}

impl<T: Into<std::io::Error>> From<T> for Error {
	/// Convert IO Errors.
	fn from(error: T) -> Self {
		Error::new(&format!("{}", error.into()))
	}
}

impl Error {
	#[inline]
	/// New Pass-Through Error.
	///
	/// This is used to contain errors bubbling up from e.g. `std::io`.
	pub fn new<S> (msg: S) -> Error
	where S: Into<String> {
		Error::Other(msg.into())
	}

	/// As String.
	///
	/// Return the Enum as a String.
	pub fn as_string(&self) -> String {
		match *self {
			Self::DoubleRun => "Flaca is already running.".to_string(),
			Self::InvalidApp => "There is no image app to handle this request.".to_string(),
			Self::InvalidPath(ref s) => format!("Invalid path: {}.", s),
			Self::IOCopy(ref from, ref to) => format!("Unable to copy {} to {}.", from, to),
			Self::IODelete(ref s) => format!("Unable to delete file: {}", s),
			Self::IOMove(ref from, ref to) => format!("Unable to move {} to {}.", from, to),
			Self::NoApp(ref s) => format!("Missing app: {}.", s),
			Self::NoApps(ref s) => format!("There are no {} apps installed.", s),
			Self::NoImages => "There are no images to work on.".to_string(),
			Self::NotJpeg(ref s) => format!("{} is not a valid JPEG.", s),
			Self::NotPng(ref s) => format!("{} is not a valid PNG.", s),
			Self::NullAlert => "Invalid alert received.".to_string(),
			Self::Other(ref s) => format!("{}", s),
		}
	}
}
