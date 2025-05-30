/*!
# Flaca: Settings
*/

use crate::{
	FlacaError,
	ImageKind,
};
use dactyl::traits::BytesToUnsigned;
use std::num::NonZeroU32;



#[derive(Debug, Clone, Copy)]
/// # Encoding Settings.
pub(crate) struct Settings {
	/// # Image Kinds (to Optimize).
	kinds: ImageKind,

	/// # Maximum Resolution.
	///
	/// Images with more pixels than this will be ignored.
	max_pixels: Option<NonZeroU32>,

	/// # Preserve Times?
	///
	/// If true, the file access and modification times will (try) to be
	/// preserved on re-save.
	preserve_times: bool,
}

impl Settings {
	#[must_use]
	/// # New Instance.
	pub(crate) const fn new() -> Self {
		Self {
			kinds: ImageKind::All,
			max_pixels: None,
			preserve_times: false,
		}
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

	/// # Preserve File Times.
	pub(super) const fn set_preserve_times(&mut self) {
		self.preserve_times = true;
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
	/// # Preserve File Times?
	pub(crate) const fn preserve_times(self) -> bool { self.preserve_times }
}
