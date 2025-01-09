/*!
# Flapfli: Katajainen and Tree-Related Business.

This module contains the Katajainen minimum-redundancy length-limited
code-writing logic — just as messy as it sounds! — as well as helpers related
to DEFLATE tree construction.
*/

mod llcl;

pub(crate) use llcl::LengthLimitedCodeLengths;
use std::{
	alloc::{
		alloc,
		Layout,
	},
	num::NonZeroU32,
	ptr::NonNull,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	DeflateSymBasic,
	TreeDist,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
	ZopfliOut,
};



/// # Fourteen is Non-Zero.
const NZ14: NonZeroU32 = NonZeroU32::new(14).unwrap();

/// # Zero-Filled Tree Counts.
const ZEROED_COUNTS_TREE: [u32; 19] = [0; 19];



/// # Calculate the Exact Tree Size (in Bits).
///
/// This returns the index (0..8) that produced the smallest size, along
/// with that size.
pub(crate) fn best_tree_size(
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> Result<(u8, NonZeroU32), ZopfliError> {
	// Merge symbols.
	let (raw_all, _, _) = tree_symbols(ll_lengths, d_lengths)?;
	let all: &[DeflateSym] = &raw_all;

	// Our targets!
	let mut best_extra = 0;
	let mut best_size = NonZeroU32::MAX;

	for extra in 0..8 {
		let cl_counts = best_tree_size_counts(all, extra);
		let cl_lengths = cl_counts.llcl()?;
		let hclen = tree_hclen(&cl_counts);

		// We can finally calculate the size!
		let mut size = (hclen as u32 + 4) * 3;
		size += cl_lengths.iter()
			.copied()
			.zip(cl_counts.iter().copied())
			.map(|(a, b)| (a as u32) * b)
			.sum::<u32>();
		size += cl_counts[16] * 2; // Extra bits.
		size += cl_counts[17] * 3;
		size += cl_counts[18] * 7;
		let size = NZ14.saturating_add(size);

		// If better, keep it!
		if size < best_size {
			best_extra = extra;
			best_size = size;
		}
	}

	// Done!
	Ok((best_extra, best_size))
}

/// # Encode Tree.
///
/// This writes the best-found tree data to `out`.
pub(crate) fn encode_tree(
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
	extra: u8,
	out: &mut ZopfliOut,
) -> Result<(), ZopfliError> {
	// Merge symbols.
	let (all, hlit, hdist) = tree_symbols(ll_lengths, d_lengths)?;

	// We'll need to store some RLE symbols and positions too.
	let mut rle: Vec<(DeflateSym, u16)> = Vec::new();

	let cl_counts = encode_tree_counts(&all, &mut rle, extra);
	let cl_lengths = cl_counts.llcl()?;
	let hclen = tree_hclen(&cl_counts);
	let cl_symbols = <[u32; 19]>::llcl_symbols(&cl_lengths);

	// Write the main lengths.
	out.add_fixed_bits::<5>(hlit as u32);
	out.add_fixed_bits::<5>(hdist as u32);
	out.add_fixed_bits::<4>(hclen as u32);

	// Write each cl_length in the jumbled DEFLATE order.
	for &o in &DeflateSym::TREE[..hclen as usize + 4] {
		out.add_fixed_bits::<3>(cl_lengths[o as usize] as u32);
	}

	// Write each symbol in order of appearance along with its extra bits,
	// if any.
	for (a, b) in rle {
		let symbol = cl_symbols[a as usize];
		out.add_huffman_bits(symbol, cl_lengths[a as usize] as u32);

		// Extra bits.
		match a {
			DeflateSym::D16 => { out.add_fixed_bits::<2>(u32::from(b)); },
			DeflateSym::D17 => { out.add_fixed_bits::<3>(u32::from(b)); },
			DeflateSym::D18 => { out.add_fixed_bits::<7>(u32::from(b)); },
			_ => {},
		}
	}

	Ok(())
}



/// # Tree Counts.
///
/// Populate and return the tree counts for `best_tree_size`.
fn best_tree_size_counts(all: &[DeflateSym], extra: u8) -> [u32; 19] {
	let mut cl_counts = ZEROED_COUNTS_TREE;
	let (use_16, use_17, use_18) = extra_bools(extra);

	let mut i = 0;
	while i < all.len() {
		let mut count = 1_u32;
		let symbol = all[i];

		/// # Helper: Special Alphabet Peekahead.
		macro_rules! special {
			($step:literal, $max:literal, $symbol:ident) => (
				while count >= $step {
					let count2 = if count < $max { count } else { $max };
					cl_counts[DeflateSym::$symbol as usize] += 1;
					count -= count2;
				}
			);
		}

		// Peek ahead to maybe save some iteration!
		let symbol_zero = symbol.is_zero();
		if use_16 || ((use_17 || use_18) && symbol_zero) {
			let mut j = i + 1;
			while j < all.len() && symbol == all[j] {
				count += 1;
				j += 1;
				i += 1;
			}

			// Repetitions of zeroes.
			if symbol_zero {
				if use_18 { special!(11, 138, D18); }
				if use_17 { special!(3, 10, D17); }
			}

			// Other symbol repetitions.
			if use_16 && count >= 4 {
				// Always count the first one as itself.
				count -= 1;
				cl_counts[symbol as usize] += 1;

				special!(3, 6, D16);
			}
		}

		// Count the current symbol and move on.
		cl_counts[symbol as usize] += count;
		i += 1;
	}

	cl_counts
}

/// # Tree Counts (Writing).
///
/// Populate and return the tree counts for `encode_tree`, as well as the RLE
/// symbol and position details.
fn encode_tree_counts(
	all: &[DeflateSym],
	rle: &mut Vec<(DeflateSym, u16)>,
	extra: u8,
) -> [u32; 19] {
	let mut cl_counts = ZEROED_COUNTS_TREE;
	let (use_16, use_17, use_18) = extra_bools(extra);

	let mut i = 0;
	while i < all.len() {
		let mut count = 1_u16;
		let symbol = all[i];

		/// # Helper: Special Alphabet Peekahead.
		macro_rules! special {
			($step:literal, $max:literal, $symbol:ident) => (
				while count >= $step {
					let count2 = if count < $max { count } else { $max };
					rle.push((DeflateSym::$symbol, count2 - $step));
					cl_counts[DeflateSym::$symbol as usize] += 1;
					count -= count2;
				}
			);
		}

		// Peek ahead to maybe save some iteration!
		let symbol_zero = symbol.is_zero();
		if use_16 || ((use_17 || use_18) && symbol_zero) {
			let mut j = i + 1;
			while j < all.len() && symbol == all[j] {
				count += 1;
				j += 1;
				i += 1;
			}

			// Repetitions of zeroes.
			if symbol_zero {
				if use_18 { special!(11, 138, D18); }
				if use_17 { special!(3, 10, D17); }
			}

			// Other symbol repetitions.
			if use_16 && count >= 4 {
				// Always count the first one as itself.
				count -= 1;
				rle.push((symbol, 0));
				cl_counts[symbol as usize] += 1;

				special!(3, 6, D16);
			}
		}

		// Count the current symbol and move on.
		for _ in 0..count { rle.push((symbol, 0)); }
		cl_counts[symbol as usize] += u32::from(count);
		i += 1;
	}

	// Done!
	cl_counts
}

/// # Extra Boolification.
///
/// Extract the use-16/17/18 bools (for tree business) from a given byte. This
/// is easy enough, but easy enough to screw up, so handy to keep in just one
/// place. ;)
const fn extra_bools(extra: u8) -> (bool, bool, bool) {
	(0 != extra & 1, 0 != extra & 2, 0 != extra & 4)
}

#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
#[expect(unsafe_code, reason = "For transmute.")]
/// # Last Non-Zero, Non-Special Count.
///
/// This method loops through the counts in the jumbled DEFLATE tree order,
/// returning the last index with a non-zero count. (The extended symbols are
/// ignored.)
const fn tree_hclen(cl_counts: &[u32; 19]) -> DeflateSymBasic {
	let mut hclen = 15;
	while cl_counts[DeflateSym::TREE[hclen + 3] as usize] == 0 {
		hclen -= 1;
		if hclen == 0 { break; }
	}
	// Safety: DeflateSymBasic covers all values between 0..=15.
	unsafe { std::mem::transmute::<u8, DeflateSymBasic>(hclen as u8) }
}

#[expect(clippy::cast_possible_truncation, reason = "False positive.")]
#[expect(unsafe_code, reason = "For transmute.")]
/// # Tree Symbols.
///
/// Drop the last two bytes from each symbol set along with up to 29
/// trailing zeroes, then merge them together (lengths then distances), and
/// return the details.
fn tree_symbols(ll_lengths: &ArrayLL<DeflateSym>, d_lengths: &ArrayD<DeflateSym>)
-> Result<(Box<[DeflateSym]>, TreeDist, TreeDist), ZopfliError> {
	// Trim non-zero symbol lengths from ll_lengths[..286], keeping the leading
	// litlen literals regardless of value.
	// literals are always kept.)
	let hlit = ll_lengths[256..286].iter()
		.rposition(|&b| ! b.is_zero())
		.map_or(TreeDist::T00, |v| {
			// Safety: the slice has length 30, and TreeDist covers 0..=29.
			unsafe { std::mem::transmute::<u8, TreeDist>(v as u8) }
		});

	// Now do the same for the distances, albeit without the literal/symbolic
	// distinction.
	let hdist = d_lengths[..30].iter()
		.rposition(|&b| ! b.is_zero())
		.map_or(TreeDist::T00, |v| {
			// Safety: the slice has length 30, and TreeDist covers 0..=29.
			unsafe { std::mem::transmute::<u8, TreeDist>(v as u8) }
		});

	// The combined length.
	let ll_len = 257 + hlit as usize;
	let d_len = 1 + hdist as usize;
	let len = ll_len + d_len;

	// We ultimately want a slice of len symbols. There are a few ways we could
	// manage this, but the most efficient is to just create a right-sized
	// layout and populate the data from pointers.

	// Safety: Rust slices and arrays are size_of::<T>() * N and share the
	// alignment of T. Length is non-zero and can't be bigger than 300ish, so
	// the layout can't fail.
	let layout = unsafe {
		Layout::from_size_align_unchecked(
			size_of::<DeflateSym>() * len,
			align_of::<DeflateSym>(),
		)
	};

	// Safety: the allocation might fail, though, so we should use the checked
	// NonNull before trying to use it!
	let nn: NonNull<DeflateSym> = NonNull::new(unsafe { alloc(layout) })
		.ok_or(zopfli_error!())?
		.cast();

	// Safety: see inline notes.
	let symbols = unsafe {
		// Copy the data into place, starting with the lengths.
		let ptr = nn.as_ptr();

		// Safety: writing 0..ll_len then ll_len..ll_len + d_len covers the
		// full allocation; everything will be initialized afterwards.
		std::ptr::copy_nonoverlapping(ll_lengths.as_ptr(), ptr, ll_len);
		std::ptr::copy_nonoverlapping(d_lengths.as_ptr(), ptr.add(ll_len), d_len);

		// Reimagine the pointer as a slice and box it up so it can be used
		// normally (and safely) hereafter.
		Box::from_raw(NonNull::slice_from_raw_parts(nn, len).as_ptr())
	};

	Ok((symbols, hlit, hdist))
}
