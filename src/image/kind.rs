/*!
# Flaca: Image Kind
*/

use std::ops::{
	BitAnd,
	BitAndAssign,
};


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
	All =  0b0011,

	/// # Jpeg.
	Jpeg = 0b0001,

	/// # Png.
	Png =  0b0010,

	/// # None.
	None = 0b0000,
}

impl BitAnd for ImageKind {
	type Output = Self;
	fn bitand(self, rhs: Self) -> Self::Output {
		Self::from((self as u8) & (rhs as u8))
	}
}

impl BitAndAssign<u8> for ImageKind {
	fn bitand_assign(&mut self, rhs: u8) {
		*self = Self::from((*self as u8) & rhs);
	}
}

impl From<u8> for ImageKind {
	fn from(src: u8) -> Self {
		match src {
			0b0011 => Self::All,
			0b0001 => Self::Jpeg,
			0b0010 => Self::Png,
			_ => Self::None,
		}
	}
}

impl ImageKind {
	/// # Is JPEG?
	pub(crate) fn is_jpeg(src: &[u8]) -> bool {
		const END: [u8; 2] = [0xFF, 0xD9];

		12 < src.len() &&
		src[..3] == [0xFF, 0xD8, 0xFF] &&
		match src[3] {
			0xE0 =>
				src[6..11] == [b'J', b'F', b'I', b'F', 0x00] ||
				src[src.len() - 2..] == END,
			0xE1 => src[6..11] ==
				[b'E', b'x', b'i', b'f', 0x00] ||
				src[src.len() - 2..] == END,
			0xE8 => src[6..12] ==
				[b'S', b'P', b'I', b'F', b'F', 0x00] ||
				src[src.len() - 2..] == END,
			0xDB | 0xE2..=0xEF => src[src.len() - 2..] == END,
			_ => false,
		}
	}

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
			"./skel/assets/jpg/02.jpg" Some(ImageKind::Jpeg),
			"./skel/assets/png/02.png" Some(ImageKind::Png),
			"./skel/assets/wolf.png" Some(ImageKind::Jpeg),
			"./skel/assets/wolf.jpg" Some(ImageKind::Png)
		);
	}
}
