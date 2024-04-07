/*!
# Flaca: Zopflipng LZ77 Store.
*/

use std::{
	mem::MaybeUninit,
	ops::{
		Deref,
		DerefMut,
	},
};
use super::{
	ZopfliCleanLZ77Store,
	ZopfliInitLZ77Store,
	ZopfliLZ77Store,
};



/// # LZ77 Store Wrapper.
///
/// This is used for instances created by Rust.
pub(super) struct LZ77Store(ZopfliLZ77Store);

impl Deref for LZ77Store {
	type Target = ZopfliLZ77Store;
	fn deref(&self) -> &Self::Target { &self.0 }
}

impl DerefMut for LZ77Store {
	fn deref_mut(&mut self) -> &mut Self::Target { &mut self.0 }
}

impl Drop for LZ77Store {
	#[allow(unsafe_code)]
	fn drop(&mut self) {
		unsafe { ZopfliCleanLZ77Store(&mut self.0); }
	}
}

impl LZ77Store {
	#[allow(unsafe_code)]
	/// # New.
	pub(super) fn new(data: *const u8) -> Self {
		unsafe {
			let mut store: MaybeUninit<ZopfliLZ77Store> = MaybeUninit::uninit();
			ZopfliInitLZ77Store(data, store.as_mut_ptr());
			Self(store.assume_init())
		}
	}

	#[allow(unsafe_code)]
	/// # Re-Initialize.
	pub(super) fn reset(&mut self, data: *const u8) {
		unsafe {
			ZopfliCleanLZ77Store(&mut self.0);
			ZopfliInitLZ77Store(data, &mut self.0);
		}
	}
}
