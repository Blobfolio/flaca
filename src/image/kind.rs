/*!
# Flaca: Image Kind
*/

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Image Kind.
///
/// This evaluates the file type from its headers, ensuring we process images
/// correctly even if they have the wrong extension (or don't process them if
/// they're bunk).
pub(super) enum ImageKind {
	/// # Jpeg.
	Jpeg,
	/// # Png.
	Png,
}

impl ImageKind {
	/// # Is JPEG?
	pub(super) fn is_jpeg(src: &[u8]) -> bool {
		12 < src.len() &&
		src[..3] == [0xFF, 0xD8, 0xFF] &&
		src[src.len() - 2..] == [0xFF, 0xD9] &&
		(
			src[3] == 0xDB ||
			src[3] == 0xEE ||
			src[3..12] == [0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01] ||
			(src[3] == 0xE1 && src[6..12] == [b'E', b'x', b'i', b'f', 0x00, 0x00])
		)
	}

	/// # Is PNG?
	pub(super) fn is_png(src: &[u8]) -> bool {
		8 < src.len() && src[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n']
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_parse() {
		macro_rules! test_kind {
			($($file:literal $ty:expr),+) => ($(
				let raw = match std::fs::read($file) {
					Ok(f) => f,
					Err(_) => panic!("Unable to open {}.", $file),
				};
				match $ty {
					Some(ImageKind::Jpeg) => {
						assert!(ImageKind::is_jpeg(&raw));
						assert!(! ImageKind::is_png(&raw));
					},
					Some(ImageKind::Png) => {
						assert!(! ImageKind::is_jpeg(&raw));
						assert!(ImageKind::is_png(&raw));
					},
					_ => {
						assert!(! ImageKind::is_jpeg(&raw));
						assert!(! ImageKind::is_png(&raw));
					},
				}
			)+);
		}

		test_kind!(
			"./skel/assets/empty.jpg" None,
			"./skel/assets/executable.sh" None,
			"./skel/assets/jpg/02.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/png/02.png" Some(ImageKind::Png),
			"./skel/assets/wolf.png" Some(ImageKind::Jpeg),
			"./skel/assets/wolf.jpg" Some(ImageKind::Png)
		);
	}
}
