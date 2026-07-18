/*!
# Flaca: Settings
*/

use crate::{
	FlacaError,
	ImageKind,
};
use dactyl::traits::BytesToUnsigned;
use std::num::NonZeroU32;



/// # Maximum LZW Alignment.
///
/// GIF dimensions are `u16`, so resolutions can only be so big.
const MAX_LZW_ALIGNMENT: NonZeroU32 = NonZeroU32::new((u16::MAX as u32) * (u16::MAX as u32)).unwrap();



#[derive(Debug, Clone, Copy)]
/// # Encoding Settings.
pub(crate) struct Settings {
	/// # Image Kinds (to Optimize).
	kinds: ImageKind,

	/// # Exhaustive LZW (GIF) Alignment.
	///
	/// Limit exhaustive LZW benching to subslices aligned to this value. None
	/// defers to frame-based logic. Zero disables.
	lzw_alignment: Option<u32>,

	/// # Maximum Resolution.
	///
	/// Images with more pixels than this will be ignored.
	max_pixels: Option<NonZeroU32>,

	/// # Flags.
	flags: u8,
}

/// # Helper: Flags.
macro_rules! flags {
	(
		$(
			$( #[doc = $meta:expr] )*
			$k:ident $v:literal $get:ident $( @set $set:ident )? $( @unset $unset:ident )?,
		)+
	) => (
		/// # Sanity Checks.
		const _: () = {
			$(
				assert!(
					0 != Settings::$k && Settings::$k.is_power_of_two(),
					"BUG: Flag(s) are not pow2!",
				); )+
			let mut all: &[u8] = &[$( $v, )+];
			while let [next, rest @ ..] = all {
				assert!(
					rest.is_empty() || *next < rest[0],
					"BUG: Flags are not unique!"
				);
				all = rest;
			}
		};

		impl Settings {
			$(
				$( #[doc = $meta] )*
				const $k: u8 = $v;
			)+

			$(
				#[must_use]
				/// # Get Flag.
				pub(crate) const fn $get(self) -> bool {
					$v == self.flags & Self::$k
				}

				$(
					/// # Set Flag.
					pub(crate) const fn $set(&mut self) {
						self.flags |= Self::$k;
					}
				)?
				$(
					/// # Unset Flag.
					pub(crate) const fn $unset(&mut self) {
						self.flags &= ! Self::$k;
					}
				)?
			)+
		}
	);
}

flags! {
	/// # Preserve Metadata?
	///
	/// If true, EXIF/etc. metadata in JPG and PNG files will be preserved.
	PRESERVE_META  0b0001 preserve_meta  @set set_preserve_meta,

	/// # Preserve Times?
	///
	/// If true, the file access and modification times will (try) to be
	/// preserved on re-save.
	PRESERVE_TIMES 0b0010 preserve_times @set set_preserve_times,

	/// # Zopfli Pass?
	///
	/// If false, PNGs will only be processed with oxipng.
	ZOPFLI         0b0100 zopfli__       @unset unset_zopfli,
}

impl Settings {
	#[must_use]
	/// # New Instance.
	pub(crate) const fn new() -> Self {
		Self {
			kinds: ImageKind::All,
			lzw_alignment: None,
			max_pixels: None,
			flags: Self::ZOPFLI,
		}
	}

	/// # Set Exhaustive LZW (GIF) Alignment.
	pub(super) const fn set_lzw_alignment(&mut self, alignment: u32) {
		// GIF dimensions are u16, so we don't need gigantic values.
		self.lzw_alignment = Some(alignment);
	}

	/// # Set Max Resolution.
	///
	/// Update the pixel limit from raw bytes (passed via CLI). The value
	/// may or may not contain a `k`/`m`/`g` prefix.
	///
	/// ## Errors
	///
	/// An error is returned if the value is invalid.
	pub(super) fn set_max_pixels_raw(&mut self, raw: &[u8])
	-> Result<(), FlacaError> {
		let multiplier: u32 =
		match raw.last() {
			Some(b'k' | b'K') => 1_000,
			Some(b'm' | b'M') => 1_000_000,
			Some(b'g' | b'G') => 1_000_000_000,
			None => return Err(FlacaError::MaxResolution),
			_ => 1,
		};

		let len = raw.len() - usize::from(multiplier != 1);
		self.max_pixels.replace(
			u32::btou(raw[..len].trim_ascii())
			.and_then(|n| n.checked_mul(multiplier))
			.and_then(NonZeroU32::new)
			.ok_or(FlacaError::MaxResolution)?
		);

		Ok(())
	}

	/// # Unset (Disable) Image Kind.
	///
	/// Disable an image kind.
	///
	/// ## Errors
	///
	/// This will return an error if no kinds remain.
	pub(super) const fn unset_kind(&mut self, kind: ImageKind)
	-> Result<(), FlacaError> {
		self.kinds.unset(kind);
		if self.kinds.is_none() { Err(FlacaError::NoImages) }
		else { Ok(()) }
	}
}

impl Settings {
	#[must_use]
	/// # Check Resolution.
	///
	/// Check that the width and height are within the limits, if any.
	pub(crate) const fn check_resolution(self, width: NonZeroU32, height: NonZeroU32)
	-> bool {
		if let Some(res) = width.checked_mul(height) {
			if let Some(max) = self.max_pixels { res.get() <= max.get() }
			else { true }
		}
		else { false }
	}

	#[must_use]
	/// # Has Kind?
	///
	/// Returns `true` if the kind is supported.
	pub(crate) const fn has_kind(self, kind: ImageKind) -> bool {
		self.kinds.contains(kind)
	}

	#[must_use]
	/// # Image Kinds.
	pub(crate) const fn kinds(self) -> ImageKind { self.kinds }

	#[must_use]
	/// # Exhaustive LZW (GIF) Alignment.
	pub(crate) const fn lzw_alignment(self) -> Option<usize> {
		if let Some(alignment) = self.lzw_alignment {
			if alignment < MAX_LZW_ALIGNMENT.get() { Some(alignment as usize) }
			else { Some(MAX_LZW_ALIGNMENT.get() as usize) }
		}
		else { None }
	}

	#[must_use]
	/// # Zopfli Pass?
	///
	/// Note that zopfli is automatically disabled when preserving metadata.
	pub(crate) const fn zopfli(self) -> bool {
		! self.preserve_meta() && self.zopfli__()
	}
}
