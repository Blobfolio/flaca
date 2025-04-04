/*!
# Flaca: Image Kind
*/

use crate::FlacaError;
use std::num::NonZeroU32;



#[repr(u8)]
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
	/// # Return the Difference.
	///
	/// Subtract `other` from `self`, returning an error if that leaves
	/// nothing.
	pub(crate) const fn diff(self, other: Self) -> Result<Self, FlacaError> {
		match other {
			Self::Jpeg if matches!(self, Self::All | Self::Png) => Ok(Self::Png),
			Self::Png if matches!(self, Self::All | Self::Jpeg) => Ok(Self::Jpeg),
			_ => Err(FlacaError::NoImages),
		}
	}

	#[expect(clippy::inline_always, reason = "For performance.")]
	#[inline(always)]
	/// # Supports JPEG?
	pub(crate) const fn supports_jpeg(self) -> bool {
		matches!(self, Self::All | Self::Jpeg)
	}

	#[expect(clippy::inline_always, reason = "For performance.")]
	#[inline(always)]
	/// # Supports PNG?
	pub(crate) const fn supports_png(self) -> bool {
		matches!(self, Self::All | Self::Png)
	}
}

impl ImageKind {
	#[expect(clippy::inline_always, reason = "For performance.")]
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

	#[expect(clippy::inline_always, reason = "For performance.")]
	#[inline(always)]
	/// # Is PNG?
	pub(crate) fn is_png(src: &[u8]) -> bool {
		8 < src.len() && src[..8] == [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1A, b'\n']
	}
}

impl ImageKind {
	/// # Width and Height.
	///
	/// Parse the image's width and height from the headers.
	pub(crate) fn jpeg_dimensions(mut raw: &[u8]) -> Option<(NonZeroU32, NonZeroU32)> {
		// We need to find the damn dimensions first!
		raw = raw.strip_prefix(&[0xFF, 0xD8])?;
		let mut depth = 0_i32;
		loop {
			//  Read the current marker, making sure it starts with FF.
			let [0xFF, sof, rest @ ..] = raw else { return None; };

			// Where we go from here depends on the SOFn marker…
			match sof {
				// C4, C8, and CC don't count, haha.
				0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF =>
					// We found it!
					if depth == 0 {
						raw = rest.get(3..)?;
						break;
					},
				0xD8 => { depth += 1; },
				0xD9 => {
					depth -= 1;
					if depth < 0 { return None; }
				},
				_ => {},
			}

			// Fast forward through the remainder of the section.
			if 2 < rest.len() {
				let len = u16::from_be_bytes([rest[0], rest[1]]);
				raw = rest.get(usize::from(len)..)?;
			}
			else { return None; }
		}

		if 4 < raw.len() {
			// Height before width for whatever reason!
			let height = NonZeroU32::new(u32::from(u16::from_be_bytes([raw[0], raw[1]])))?;
			let width = NonZeroU32::new(u32::from(u16::from_be_bytes([raw[2], raw[3]])))?;
			Some((width, height))
		}
		else { None }
	}

	/// # Width and Height.
	///
	/// Parse the image's width and height from the headers.
	pub(crate) fn png_dimensions(raw: &[u8]) -> Option<(NonZeroU32, NonZeroU32)> {
		if raw.len() > 16 + 8 && raw[12..16].eq(b"IHDR") {
			let width = NonZeroU32::new(u32::from_be_bytes([raw[16], raw[17], raw[18], raw[19]]))?;
			let height = NonZeroU32::new(u32::from_be_bytes([raw[20], raw[21], raw[22], raw[23]]))?;
			Some((width, height))
		}
		else { None }
	}
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn t_jpeg_dimensions() {
		let raw: &[(&str, u32, u32)] = &[
			("../skel/assets/jpg/01.jpg", 1934, 1088),
			("../skel/assets/jpg/02.jpg", 2048, 1536),
			("../skel/assets/jpg/03.jpg", 1324, 2095),
			("../skel/assets/jpg/04.jpg", 1280, 800),
			("../skel/assets/jpg/05.jpg", 2400, 3000),
			("../skel/assets/jpg/06.jpg", 6240, 4160),
			("../skel/assets/jpg/07.jpg", 1460, 730),
			("../skel/assets/jpg/08.jpg", 1024, 512),
			("../skel/assets/jpg/09.jpg", 3200, 1800),
			("../skel/assets/jpg/10.jpg", 994, 663),
			("../skel/assets/jpg/11.jpg", 994, 627),
			("../skel/assets/jpg/12.jpg", 1000, 750),
			("../skel/assets/jpg/13.jpg", 72, 48),
			("../skel/assets/jpg/14.jpg", 72, 48),
			("../skel/assets/jpg/15.jpg", 4000, 3000),
			("../skel/assets/jpg/16.jpg", 3264, 2448),
			("../skel/assets/jpg/17.jpg", 512, 512),
			("../skel/assets/jpg/18.jpg", 512, 512),
			("../skel/assets/jpg/19.jpg", 512, 512),
			("../skel/assets/jpg/20.jpg", 512, 512),
			("../skel/assets/jpg/21.jpg", 720, 462),
			("../skel/assets/jpg/22.jpg", 267, 150),
			("../skel/assets/jpg/23.jpg", 330, 313),
			("../skel/assets/jpg/24.jpg", 1076, 1500),
			("../skel/assets/wolf.png", 600, 800),

			// And because JPEGs are so weird, let's double-check our work
			// against some reference assets from the imagesize crate too!
			("../skel/dimensionality/size01.jpg", 1500, 844),
			("../skel/dimensionality/size02.jpg", 1360, 1904),
			("../skel/dimensionality/size03.jpg", 690, 298),
			("../skel/dimensionality/size04.jpg", 3047, 2008),
			("../skel/dimensionality/size05.jpg", 4980, 3321),
			("../skel/dimensionality/size06.jpg", 2995, 1998),
			("../skel/dimensionality/size07.jpg", 1080, 1080),
		];
		for &(file, w1, h1) in raw {
			let Ok(raw) = std::fs::read(file) else { panic!("Unable to open {file}."); };
			let Some((w2, h2)) = ImageKind::jpeg_dimensions(&raw) else {
				panic!("Unable to parse dimensions from {file}.");
			};
			assert_eq!(w1, w2.get(), "Width mismatch {w1} / {w2} for {file}.");
			assert_eq!(h1, h2.get(), "Height mismatch {h1} / {h2} for {file}.");
		}
	}

	#[test]
	fn t_png_dimensions() {
		let raw: &[(&str, u32, u32)] = &[
			("../skel/assets/png/01.png", 800, 500),
			("../skel/assets/png/02.png", 500, 516),
			("../skel/assets/png/03.png", 1024, 576),
			("../skel/assets/png/04.png", 640, 400),
			("../skel/assets/png/05.png", 2800, 2066),
			("../skel/assets/png/06.png", 1024, 790),
			("../skel/assets/png/poe.png", 640, 440),
			("../skel/assets/png/small-bw.png", 50, 50),
			("../skel/assets/png/small-bwa.png", 50, 50),
			("../skel/assets/png/small.png", 32, 32),
			("../skel/assets/wolf.jpg", 600, 800),
		];
		for &(file, w1, h1) in raw {
			let Ok(raw) = std::fs::read(file) else { panic!("Unable to open {file}."); };
			let Some((w2, h2)) = ImageKind::png_dimensions(&raw) else {
				panic!("Unable to parse dimensions from {file}.");
			};
			assert_eq!(w1, w2.get(), "Width mismatch {w1} / {w2} for {file}.");
			assert_eq!(h1, h2.get(), "Height mismatch {h1} / {h2} for {file}.");
		}
	}

	#[test]
	#[expect(clippy::cognitive_complexity, reason = "It is what it is.")]
	fn t_parse() {
		macro_rules! test_kind {
			($($file:literal $ty:expr),+) => ($(
				let Ok(raw) = std::fs::read($file) else {
					panic!("Unable to open {}.", $file);
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
			"../skel/assets/empty.jpg" None,
			"../skel/assets/executable.sh" None,
			"../skel/assets/herring.png" None,
			"../skel/assets/jpg/01.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/02.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/03.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/04.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/05.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/06.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/07.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/08.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/09.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/10.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/11.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/12.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/13.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/14.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/15.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/16.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/17.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/18.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/19.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/20.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/21.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/22.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/23.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/jpg/24.jpg" Some(ImageKind::Jpeg),
			"../skel/assets/png/01.png" Some(ImageKind::Png),
			"../skel/assets/png/02.png" Some(ImageKind::Png),
			"../skel/assets/png/03.png" Some(ImageKind::Png),
			"../skel/assets/png/04.png" Some(ImageKind::Png),
			"../skel/assets/png/05.png" Some(ImageKind::Png),
			"../skel/assets/png/06.png" Some(ImageKind::Png),
			"../skel/assets/png/poe.png" Some(ImageKind::Png),
			"../skel/assets/png/small-bw.png" Some(ImageKind::Png),
			"../skel/assets/png/small-bwa.png" Some(ImageKind::Png),
			"../skel/assets/png/small.png" Some(ImageKind::Png),
			"../skel/assets/wolf.jpg" Some(ImageKind::Png),
			"../skel/assets/wolf.png" Some(ImageKind::Jpeg)
		);
	}
}
