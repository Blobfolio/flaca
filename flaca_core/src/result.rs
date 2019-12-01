/*!
# Result

This holds the result of an image compression attempt.
*/

use crate::image::ImageKind;
use std::path::PathBuf;



#[derive(Debug, Clone)]
/// A Compression Result.
pub struct FlacaResult {
	/// The file path.
	pub path: PathBuf,
	/// The kind of file.
	pub kind: ImageKind,
	/// The duration of execution (in milliseconds).
	pub duration: f64,
	/// The size before and after compression.
	pub size: (usize, usize),
}

impl Default for FlacaResult {
	/// Default.
	fn default() -> FlacaResult {
		FlacaResult {
			path: PathBuf::new(),
			kind: ImageKind::None,
			duration: 0.0,
			size: (0, 0),
		}
	}
}

impl FlacaResult {
	/// Bytes Saved.
	///
	/// Returns the total bytes saved from all ImageApp passes.
	pub fn saved(&self) -> usize {
		let (before, after) = self.size;
		if 0 < after && after < before {
			before - after
		}
		else {
			0
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// Test Saved.
	fn test_saved() {
		// A little smaller.
		let result: FlacaResult = FlacaResult {
			size: (5000, 4000),
			..FlacaResult::default()
		};
		assert_eq!(result.saved(), 1000);

		// No change.
		let result: FlacaResult = FlacaResult {
			size: (5000, 5000),
			..FlacaResult::default()
		};
		assert_eq!(result.saved(), 0);

		// An impossible backward regression.
		let result: FlacaResult = FlacaResult {
			size: (5000, 6000),
			..FlacaResult::default()
		};
		assert_eq!(result.saved(), 0);
	}
}
