/*!
# Error

Because _every_ Rust crate needs its very own interpretation of an error
response!
*/

use std::error::Error;
use std::fmt;



#[derive(Debug, Clone, Eq, PartialEq)]
/// An Error!
pub enum FlacaError {
	/// Flaca is already running.
	AlreadyRunning,
	/// Duplicate file.
	DuplicatePath,
	/// The log is empty.
	EmptyLog,
	/// The queue is empty.
	EmptyQueue,
	/// A file could not be copied.
	FileCopy,
	/// A path is invalid for whatever purpose.
	InvalidPath,
	/// A required ImageApp could not be found.
	MissingWorker,
	/// A path is not executable.
	NotExecutable,
	/// A path is not an image.
	NotImage,
	/// A path is not a JPEG.
	NotJpeg,
	/// A path is not a PNG.
	NotPng,
	/// There are no images.
	NoImages,
	/// There are no available ImageApps.
	NoWorkers,
	/// There are no available JPEG ImageApps.
	NoJpegWorkers,
	/// There are no available PNG ImageApps.
	NoPngWorkers,
	/// This is a generic pass-through error used to contain third-party
	/// errors bubbling up from e.g. `std::io`.
	Other(String),
}

impl Error for FlacaError {
	/// Description.
	fn description(&self) -> &str {
		""
	}
}

impl fmt::Display for FlacaError {
	#[inline]
	/// Display.
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		f.write_str(&self.as_string())
	}
}

impl<T: Into<std::io::Error>> From<T> for FlacaError {
	/// Convert IO Errors.
	fn from(error: T) -> Self {
		FlacaError::new(&format!("{}", error.into()))
	}
}

impl FlacaError {
	#[inline]
	/// New Pass-Through Error.
	///
	/// This is used to contain errors bubbling up from e.g. `std::io`.
	pub fn new<S> (msg: S) -> FlacaError
	where S: Into<String> {
		FlacaError::Other(msg.into())
	}

	/// As String.
	///
	/// Return the Enum as a String.
	pub fn as_string(&self) -> String {
		match *self {
			Self::AlreadyRunning  => "Please wait for the current job(s) to finish.",
			Self::DuplicatePath => "A duplicate file is already in the queue.",
			Self::EmptyLog  => "No matching logs have been recorded.",
			Self::EmptyQueue  => "There is nothing to do.",
			Self::FileCopy  => "Unable to copy the file.",
			Self::InvalidPath  => "Invalid path.",
			Self::MissingWorker  => "Missing image optimizer.",
			Self::NoImages => "No images were found.",
			Self::NoJpegWorkers => "No JPEG image optimizers have been specified.",
			Self::NoPngWorkers => "No PNG image optimizers have been specified.",
			Self::NotExecutable => "Not an executable path.",
			Self::NotImage => "Not an image file.",
			Self::NotJpeg => "Not a JPEG file.",
			Self::NotPng => "Not a PNG file.",
			Self::NoWorkers => "No image optimizers are available to compress images of this type.",
			Self::Other(ref s) => s,
		}.to_string()
	}
}
