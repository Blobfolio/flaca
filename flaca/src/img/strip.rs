/*!
# Flaca: Metadata Stripping.
*/

use super::ImageKind;



/// # Helper: Stripable Markers.
///
/// This just helps enforce consistency.
macro_rules! jpeg_meta_markers {
	() => ( 0xE1..=0xED | 0xEF | 0xFE );
}


/// # Helper: JPEG Marker enum.
macro_rules! jpeg_markers {
	( $( $k:ident $v:literal $doc:literal, )+ ) => (
		#[repr(u8)]
		#[derive(Debug, Clone, Copy, Eq, PartialEq)]
		/// # JPEG Segment Markers.
		///
		/// This enum is used to house the JPEG markers supported by the
		/// `MozJPEG` decoder, and a few simple methods for dealing with them.
		pub(super) enum JpegMarker {
			$(
				#[doc = concat!("# ", $doc)]
				$k = $v,
			)+
		}

		/// # Sanity checks.
		const _: () = {
			$(
				// Make sure we got meta markers right.
				assert!(
					matches!(
						JpegMarker::$k,
						JpegMarker::App1 | JpegMarker::App2 | JpegMarker::App3 |
						JpegMarker::App4 | JpegMarker::App5 | JpegMarker::App6 |
						JpegMarker::App7 | JpegMarker::App8 | JpegMarker::App9 |
						JpegMarker::App10 | JpegMarker::App11 | JpegMarker::App12 |
						JpegMarker::App13 | JpegMarker::App15 | JpegMarker::Com
					) == JpegMarker::$k.metadata(),
				);

				// And parameterless ones.
				assert!(
					matches!(
						JpegMarker::$k,
						JpegMarker::Tem |
						JpegMarker::Rst0 | JpegMarker::Rst1 | JpegMarker::Rst2 |
						JpegMarker::Rst3 | JpegMarker::Rst4 | JpegMarker::Rst5 |
						JpegMarker::Rst6 | JpegMarker::Rst7 |
						JpegMarker::Soi | JpegMarker::Eoi
					) == JpegMarker::$k.parameterless(),
				);
			)+

			// Make sure they're sorted by value.
			let mut last = JpegMarker::ZERO;
			let mut all: &[JpegMarker] = &[$( JpegMarker::$k, )+];
			while let [next, rest @ ..] = all {
				let next = *next as u8;
				assert!(last < next);
				last = next;
				all = rest;
			}
		};

		impl JpegMarker {
			/// # Zero Byte.
			const ZERO: u8 = 0x00;

			/// # All Byte.
			const ALL: u8 =  0xFF;

			#[must_use]
			/// # Strip Metadata Segments.
			///
			/// Build and return a clone of the original JPEG image with
			/// `APP1..=APP13`, `APP15` and `COM` segments stripped out.
			///
			/// If no savings are found or the resulting image cannot be
			/// decoded or is somehow different than the input, `None` is
			/// returned.
			pub(super) fn strip_metadata(raw: &[u8]) -> Option<Vec<u8>> {
				// Short circuit.
				if
					// Not a JPEG.
					! ImageKind::is_jpeg(raw) ||
					// No strippable marker sequences, valid or otherwise.
					! raw.array_windows::<2>().any(|pair| matches!(pair, [Self::ALL, jpeg_meta_markers!()]))
				{
					return None;
				}

				// Build up a new image without the metadata segments.
				let mut out = Vec::with_capacity(raw.len());
				for segment in JpegSegmentIter::new(raw) {
					out.extend_from_slice(segment.ok()?);
				}

				// Keep it if smaller and uncorrupted!
				if
					out.len() < raw.len() &&
					ImageKind::is_jpeg(&out) &&
					same_jpeg_pixels(raw, &out)
				{
					Some(out)
				}
				else { None }
			}

			#[must_use]
			/// # From Byte.
			const fn from_byte(byte: u8) -> Option<Self> {
				match byte {
					$( $v => Some(Self::$k), )+
					_ => None,
				}
			}

			#[must_use]
			/// # Parameterless?
			///
			/// Returns true if the marker has no data, i.e. `Tem`, `RstN`,
			/// `Soi`, and `Eoi`.
			const fn parameterless(self) -> bool {
				matches!(self as u8, 0x01 | 0xD0..=0xD9)
			}

			#[must_use]
			/// # Is Metadata?
			///
			/// Returns `true` for `APP1..=APP13`, `APP15`, and `COM`
			/// segments.
			const fn metadata(self) -> bool {
				matches!(self as u8, jpeg_meta_markers!())
			}
		}
	);
}

// MozJPEG decoding marker support:
// https://github.com/mozilla/mozjpeg/blob/master/jdmarker.c

jpeg_markers! {
	Tem   0x01 "Temporary?",
	Sof0  0xC0 "Start of Frame (Baseline DCT).",
	Sof1  0xC1 "Start of Frame (Sequential DCT).",
	Sof2  0xC2 "Start of Frame (Progressive DCT).",
	Sof3  0xC3 "Start of Frame (Lossless).",
	Dht   0xC4 "Define Huffman Table(s).",
	// Sof5  0xC5 "Start of Frame (Differential Sequential DCT).",
	// Sof6  0xC6 "Start of Frame (Differential Progressive DCT).",
	// Sof7  0xC7 "Start of Frame (Differential Lossless).",
	// Jpg   0xC8 "JPG (Extension)."
	Sof9  0xC9 "Start of Frame (Arithmetic Sequential DCT).",
	Sof10 0xCA "Start of Frame (Arithmetic Progressive DCT).",
	Sof11 0xCB "Start of Frame (Arithmetic Lossless).",
	Dac   0xCC "Define Arithmetic Conditions.",
	// Sof13 0xCD "Start of Frame (Arithmetic, Differential Sequential DCT).",
	// Sof14 0xCE "Start of Frame (Arithmetic, Differential Progressive DCT).",
	// Sof15 0xCF "Start of Frame (Arithmetic, Differential Lossless).",

	Rst0  0xD0 "Restart.",
	Rst1  0xD1 "Restart.",
	Rst2  0xD2 "Restart.",
	Rst3  0xD3 "Restart.",
	Rst4  0xD4 "Restart.",
	Rst5  0xD5 "Restart.",
	Rst6  0xD6 "Restart.",
	Rst7  0xD7 "Restart.",
	Soi   0xD8 "Start of Image.",
	Eoi   0xD9 "End of Image.",
	Sos   0xDA "Start of Scan.",
	Dqt   0xDB "Define Quantization Table(s).",
	Dnl   0xDC "Define Number of Lines.",
	Dri   0xDD "Define Restart Interval.",
	// Dhp   0xDE "Define Hierarchical Progression.",
	// Exp   0xDF "Expand Reference Components.",

	App0  0xE0 "Application (JFIF).",
	App1  0xE1 "Application (EXIF, XMP).",
	App2  0xE2 "Application (ICC).",
	App3  0xE3 "Application.",
	App4  0xE4 "Application.",
	App5  0xE5 "Application.",
	App6  0xE6 "Application.",
	App7  0xE7 "Application.",
	App8  0xE8 "Application (SPIFF).",
	App9  0xE9 "Application.",
	App10 0xEA "Application.",
	App11 0xEB "Application.",
	App12 0xEC "Application.",
	App13 0xED "Application (Adobe).",
	App14 0xEE "Application (Adobe).",
	App15 0xEF "Application (ignore).",

	// Jpg0  0xF0 "JPEG (Extension).",
	// Jpg13 0xFD "JPEG (Extension).",
	Com   0xFE "Comment.",
}



#[derive(Debug)]
/// # JPEG Segment Iterator.
///
/// This iterator splits the raw bytes of a JPEG image into its constituent
/// segments, yielding the non-metadata ones in order.
struct JpegSegmentIter<'a> {
	/// # Raw Source Image.
	src: &'a [u8],

	/// # Current Position.
	pos: usize,

	/// # Start of Image should be first.
	soi: bool,

	/// # End of Image?
	eoi: bool,
}

impl<'a> JpegSegmentIter<'a> {
	#[must_use]
	/// # New.
	const fn new(src: &'a[u8]) -> Self {
		Self { src, pos: 0, soi: false, eoi: false }
	}
}

impl JpegSegmentIter<'_> {
	#[must_use]
	/// # Segment Length.
	///
	/// Parse the first two bytes from `raw` (at the provided offset) as a
	/// bigendian length (including those two bytes), returning it if valid for
	/// the size of the slice.
	const fn segment_len(&self) -> Option<usize> {
		// We need at least two bytes.
		if self.pos + 1 < self.src.len() {
			let len = u16::from_be_bytes([self.src[self.pos], self.src[self.pos + 1]]) as usize;

			// Return the length if in range.
			if 2 <= len && self.pos + len <= self.src.len() { Some(len) }
			// Nope.
			else { None }
		}
		// Nope.
		else { None }
	}
}

impl<'a> Iterator for JpegSegmentIter<'a> {
	type Item = Result<&'a [u8], ()>;

	fn next(&mut self) -> Option<Self::Item> {
		while self.pos < self.src.len() {
			// If we've already returned an End of Image marker, dump the
			// remainder all in one go.
			if self.eoi {
				let out = &self.src[self.pos..];
				self.pos = self.src.len();
				return Some(Ok(out));
			}

			// Skip padding.
			let mut any = false;
			let mut segment_start = self.pos;
			while self.pos < self.src.len() && self.src[self.pos] == JpegMarker::ALL {
				segment_start = self.pos;
				self.pos += 1;
				any = true;
			}

			// Abort if we didn't have at least one ALL byte.
			if ! any || self.src.len() <= self.pos {
				self.pos = self.src.len();
				return Some(Err(()));
			}

			// Check the marker.
			let Some(marker) = JpegMarker::from_byte(self.src[self.pos]) else {
				self.pos = self.src.len();
				return Some(Err(()));
			};
			self.pos += 1;

			// Start of image?
			if matches!(marker, JpegMarker::Soi) {
				// There can be only one!
				if self.soi {
					self.pos = self.src.len();
					return Some(Err(()));
				}
				self.soi = true;
			}
			// Start of image must come first.
			else if ! self.soi {
				self.pos = self.src.len();
				return Some(Err(()));
			}

			// Find the end.
			let mut segment_end = self.pos;
			if ! marker.parameterless() {
				let Some(len) = self.segment_len() else {
					self.pos = self.src.len();
					return Some(Err(()));
				};
				self.pos += len;
				segment_end = self.pos;

				// SOS segments include entropy-coded image data, but for
				// some terrible reason there is no length field to reference.
				// There's nothing for it but to read ahead to find the next
				// (probable) marker — excluding rescans — or EOF. Because
				// data can
				if matches!(marker, JpegMarker::Sos) {
					if let Some(more) = self.src.array_windows::<2>()
						.skip(self.pos)
						.position(|pair| post_stream_marker(*pair))
					{
						self.pos += more;
					}
					else { self.pos = self.src.len(); }
					segment_end = self.pos;
				}
			}

			// Stop here if the marker is worth keeping!
			if ! marker.metadata() {
				// Far be it from us to validate data trailing the EOI marker.
				// On the next pass, we'll just return whatever's left.
				if matches!(marker, JpegMarker::Eoi) { self.eoi = true; }

				// Sanity checks.
				if segment_start <= segment_end && segment_end <= self.src.len() {
					return Some(Ok(&self.src[segment_start..segment_end]));
				}

				// This shouldn't happen.
				self.pos = self.src.len();
				return Some(Err(()));
			}

			// Back around again.
		}

		// We're out of segments!
		None
	}
}

#[must_use]
/// # End of Stream Marker.
///
/// The gibberish following an SOS segment is assumed to be entropy-coded image
/// data unless/until a sequence of `0xFF + !(0x00 | Rst)` is found.
const fn post_stream_marker(pair: [u8; 2]) -> bool {
	pair[0] == JpegMarker::ALL &&
	! matches!(pair[1], JpegMarker::ZERO | 0xD0..=0xD7)
}

#[must_use]
/// # Same JPEG Pixels?
///
/// Verify two images can A) be decoded and B) have identical pixels.
///
/// This is not particularly efficient, but given the relative naivete of our
/// segment traversal, is worth checking to avoid accidental corruption.
fn same_jpeg_pixels(a: &[u8], b: &[u8]) -> bool {
	use image::ImageFormat;
	let Ok(b) = image::load_from_memory_with_format(b, ImageFormat::Jpeg) else {
		return false;
	};
	let b = b.to_rgb8();

	let Ok(a) = image::load_from_memory_with_format(a, ImageFormat::Jpeg) else {
		return false;
	};
	let a = a.to_rgb8();

	// Dimensions and pixels should match!
	a.width() == b.width() &&
	a.height() == b.height() &&
	a.pixels().eq(b.pixels())
}



#[cfg(test)]
mod tests {
	use super::*;
	use image::{
		RgbImage,
		ImageFormat,
	};

	#[test]
	fn t_post_stream_marker() {
		for i in u8::MIN..=u8::MAX {
			assert_ne!(
				// Everything but zero and Rst are terminators in this context.
				(
					i == JpegMarker::ZERO ||
					matches!(
						JpegMarker::from_byte(i),
						Some(
							JpegMarker::Rst0 | JpegMarker::Rst1 | JpegMarker::Rst2 |
							JpegMarker::Rst3 | JpegMarker::Rst4 | JpegMarker::Rst5 |
							JpegMarker::Rst6 | JpegMarker::Rst7
						)
					)
				),
				post_stream_marker([JpegMarker::ALL, i])
			)
		}
	}

	#[test]
	fn t_strip_jpeg() {
		/// # Decode JPEG.
		fn decode(raw: &[u8]) -> Option<RgbImage> {
			let img = image::load_from_memory_with_format(raw, ImageFormat::Jpeg)
				.ok()?
				.to_rgb8();

			// Verify the dimensions.
			assert_eq!(img.width(), 1024);
			assert_eq!(img.height(), 512);

			// Simplify (to RGB) and return.
			Some(img)
		}

		// Original image.
		let raw = std::fs::read("../skel/assets/jpg/08.jpg")
			.expect("Unable to read 08.jpg");

		// Strip it.
		let stripped = JpegMarker::strip_metadata(&raw).expect("Stripping 08.jpg failed!");

		// The stripped version should be slightly smaller.
		assert!(stripped.len() < raw.len());

		// Decode both images.
		let raw = decode(&raw).expect("Unable to decode 08.jpg");
		let stripped = decode(&stripped).expect("Unable to decode (stripped) 08.jpg");

		// The pixels should be identical!
		assert!(raw.pixels().eq(stripped.pixels()));
	}
}
