/*!
# Flaca: Image Kind
*/

#[repr(u8)]
#[allow(clippy::redundant_pub_crate)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
/// # Image Kind.
///
/// This evaluates the file type from its headers, ensuring we process images
/// correctly even if they have the wrong extension (or don't process them if
/// they're bunk).
pub(crate) enum ImageKind {
	/// # All.
	All,

	/// # Jpeg.
	Jpeg,

	/// # Png.
	Png,
}

impl ImageKind {
	#[allow(clippy::inline_always)]
	#[inline(always)]
	/// # Supports JPEG?
	pub(crate) const fn supports_jpeg(self) -> bool {
		matches!(self, Self::All | Self::Jpeg)
	}

	#[allow(clippy::inline_always)]
	#[inline(always)]
	/// # Supports PNG?
	pub(crate) const fn supports_png(self) -> bool {
		matches!(self, Self::All | Self::Png)
	}
}

impl ImageKind {
	#[allow(clippy::inline_always)]
	#[inline(always)]
	/// # Is JPEG?
	pub(crate) fn is_jpeg(src: &[u8]) -> bool {
		12 < src.len() &&
		src[..3] == [0xFF, 0xD8, 0xFF] &&
		(
			(src[3] == 0xE0 && src[6..11] == [b'J', b'F', b'I', b'F', 0x00]) ||
			(src[3] == 0xE1 && src[6..11] == [b'E', b'x', b'i', b'f', 0x00]) ||
			(src[3] == 0xE8 && src[6..12] == [b'S', b'P', b'I', b'F', b'F', 0x00]) ||
			(matches!(src[3], 0xDB | 0xE0..=0xEF) && src[src.len() - 2..] == [0xFF, 0xD9])
		)
	}

	#[allow(clippy::inline_always)]
	#[inline(always)]
	/// # Is PNG?
	pub(crate) fn is_png(src: &[u8]) -> bool {
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
			"./skel/assets/herring.png" None,
			"./skel/assets/jpg/01.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/02.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/03.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/04.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/05.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/06.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/07.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/08.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/09.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/10.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/11.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/12.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/13.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/14.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/15.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/16.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/17.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/18.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/19.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/20.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/21.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/22.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/jpg/23.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/png/01.png" Some(ImageKind::Png),
			"./skel/assets/png/02.png" Some(ImageKind::Png),
			"./skel/assets/png/03.png" Some(ImageKind::Png),
			"./skel/assets/png/04.png" Some(ImageKind::Png),
			"./skel/assets/png/05.png" Some(ImageKind::Png),
			"./skel/assets/png/06.png" Some(ImageKind::Png),
			"./skel/assets/png/poe.png" Some(ImageKind::Png),
			"./skel/assets/png/small-bw.png" Some(ImageKind::Png),
			"./skel/assets/png/small-bwa.png" Some(ImageKind::Png),
			"./skel/assets/png/small.png" Some(ImageKind::Png),
			"./skel/assets/wolf.jpg" Some(ImageKind::Png),
			"./skel/assets/wolf.png" Some(ImageKind::Jpeg)
		);
	}
}
