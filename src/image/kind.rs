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
	/// Jpeg.
	Jpeg,
	/// Png.
	Png,
}

impl ImageKind {
	/// # Parse Kind from Magic Bytes.
	pub(super) fn parse(src: &[u8]) -> Option<Self> {
		// If the source is big enough for headers, keep going!
		if src.len() > 12 {
			// PNG has just one way to be!
			if src[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n'] {
				return Some(Self::Png);
			}

			// JPEG has a lot of different possible headers. They all start and
			// end the same way, but have some differences in the middle.
			if
				src[..3] == [0xFF, 0xD8, 0xFF] &&
				src[src.len() - 2..] == [0xFF, 0xD9] &&
				(
					src[3] == 0xDB ||
					src[3] == 0xEE ||
					(src[3..12] == [0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01]) ||
					(src[3] == 0xE1 && src[6..12] == [b'E', b'x', b'i', b'f', 0x00, 0x00])
				)
			{
				return Some(Self::Jpeg);
			}
		}

		None
	}
}



#[test]
fn t_parse() {
	macro_rules! test_kind {
		($($file:literal $ty:expr),+) => ($(
			assert_eq!(
				std::fs::read($file).ok().and_then(|x| ImageKind::parse(&x)),
				$ty
			);
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
