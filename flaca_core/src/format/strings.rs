/*!
# Formatting Helpers: Strings
*/

use std::ffi::{OsStr, OsString};



/// From OsStr(ing).
pub fn from_os_string<S> (text: S) -> String
where S: Into<OsString> {
	text.into().to_str().unwrap_or("").to_string()
}

/// To OsString.
pub fn to_os_string<S> (text: S) -> OsString
where S: Into<String> {
	OsStr::new(&text.into()).to_os_string()
}

/// Find Padding Needed.
fn pad_diff<S> (text: S, len: usize) -> usize
where S: Into<String> {
	let text_len: usize = text.into().len();
	match text_len >= len {
		true => 0,
		false => len - text_len,
	}
}

/// Pad String On Left.
pub fn pad_left<S>(text: S, pad_len: usize, pad_fill: u8) -> String
where S: Into<String> {
	let text = text.into();

	match pad_diff(&text, pad_len) {
		0 => text,
		x => format!(
			"{}{}",
			String::from_utf8(vec![pad_fill; x]).unwrap_or("".to_string()),
			text,
		),
	}
}

/// Pad String On Right.
pub fn pad_right<S>(text: S, pad_len: usize, pad_fill: u8) -> String
where S: Into<String> {
	let text = text.into();

	match pad_diff(&text, pad_len) {
		0 => text,
		x => format!(
			"{}{}",
			text,
			String::from_utf8(vec![pad_fill; x]).unwrap_or("".to_string()),
		),
	}
}

/// Shorten String From Left (Keeping Right).
pub fn shorten_left<S>(text: S, len: usize) -> String
where S: Into<String> {
	match len {
		0 => "".to_string(),
		1 => "…".to_string(),
		_ => {
			// Pull text details.
			let text = text.into();
			let text_len: usize = text.len();

			// Shorten away!
			match text_len <= len {
				true => text,
				false => {
					let short: String = text.chars()
						.skip(text_len - len + 1)
						.collect();
					format!("…{}", short.trim())
				},
			}
		}
	}
}

/// Shorten String From Right (Keeping Left).
pub fn shorten_right<S>(text: S, len: usize) -> String
where S: Into<String> {
	match len {
		0 => "".to_string(),
		1 => "…".to_string(),
		_ => {
			// Pull text details.
			let text = text.into();
			let text_len: usize = text.len();

			// Shorten away!
			match text_len <= len {
				true => text,
				false => {
					let short: String = text.chars()
						.take(len - 1)
						.collect();
					format!("{}…", short.trim())
				},
			}
		}
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// Test Pad Left.
	fn test_pad_left() {
		let data = vec![
			("apples", 5, b' ', "apples"),
			("apples", 10, b' ', "    apples"),
			("apples", 10, b'#', "####apples"),
		];

		for d in data.as_slice() {
			let (text, pad_len, pad_fill, expected) = *d;
			assert_eq!(pad_left(text, pad_len, pad_fill), expected);
		}
	}

	#[test]
	/// Test Pad Right.
	fn test_pad_right() {
		let data = vec![
			("apples", 5, b' ', "apples"),
			("apples", 10, b' ', "apples    "),
			("apples", 10, b'#', "apples####"),
		];

		for d in data.as_slice() {
			let (text, pad_len, pad_fill, expected) = *d;
			assert_eq!(pad_right(text, pad_len, pad_fill), expected);
		}
	}

	#[test]
	/// Test Shorten Left.
	fn test_shorten_left() {
		let data = vec![
			("apples", 0, ""),
			("apples", 3, "…es"),
			("apples", 5, "…ples"),
			("apples", 6, "apples"),
			("apples", 7, "apples"),
		];

		for d in data.as_slice() {
			let (text, len, expected) = *d;
			assert_eq!(shorten_left(text, len), expected);
		}
	}

	#[test]
	/// Test Shorten Right.
	fn test_shorten_right() {
		let data = vec![
			("apples", 0, ""),
			("apples", 3, "ap…"),
			("apples", 5, "appl…"),
			("apples", 6, "apples"),
			("apples", 7, "apples"),
		];

		for d in data.as_slice() {
			let (text, len, expected) = *d;
			assert_eq!(shorten_right(text, len), expected);
		}
	}
}
