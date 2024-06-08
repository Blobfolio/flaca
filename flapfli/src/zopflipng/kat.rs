/*!
# Flapfli: Katajainen and Tree-Related Business.

This module contains the Katajainen minimum-redundancy length-limited
code-writing logic — just as messy as it sounds! — as well as helpers related
to DEFLATE tree construction.
*/

use std::{
	alloc::{
		alloc,
		handle_alloc_error,
		Layout,
	},
	cell::Cell,
	cmp::Ordering,
	num::NonZeroU32,
	ptr::NonNull,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
	ZopfliOut,
};



#[allow(unsafe_code)]
const NZ1: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) };

#[allow(unsafe_code)]
const NZ2: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };



thread_local!(
	/// # Shared Node Scratch.
	///
	/// The length-limited-code-length methods need to temporarily store
	/// thousands of `Node` objects. Using a thread-local share for that cuts
	/// way down on the number of allocations we'd otherwise have to make!
	static SCRATCH: KatScratch = KatScratch::new()
);



#[allow(clippy::wildcard_imports)]
mod sealed {
	use super::*;

	#[allow(private_bounds, private_interfaces, unreachable_pub)]
	/// # Length Limited Code Lengths (Private).
	///
	/// This sealed trait provides the core LLCL-related functionality for the
	/// three different count sizes implementing `LengthLimitedCodeLengths`,
	/// keeping them from cluttering the public ABI.
	pub trait LengthLimitedCodeLengthsSealed<const MAXBITS: usize, const N: usize> {
		#[allow(unsafe_code)]
		/// # Crunch the Code Lengths.
		///
		/// This method serves as the closure for the caller's call to
		/// `SCRATCH.with_borrow_mut()`.
		fn _llcl<'a>(
			frequencies: &'a [u32; N],
			bitlengths: &'a [Cell<DeflateSym>; N],
			nodes: &KatScratch
		) -> Result<(), ZopfliError> {
			let leaves = nodes.leaves(frequencies, bitlengths);
			if leaves.len() <= 2 {
				for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
				return Ok(());
			}

			// Set up the lists.
			let lists = nodes.lists(
				MAXBITS.min(leaves.len() - 1),
				leaves[0].frequency,
				leaves[1].frequency,
			);
			if lists.len() < 2 {
				// Safety: we'll always have `MAXBITS.min(leaves.len() - 1)` lists, but
				// the compiler might not realize that without inlining.
				unsafe { core::hint::unreachable_unchecked(); }
			}

			// In the last list, (2 * len_leaves - 2) active chains need to be
			// created. We have two already from initialization; each boundary_pm run
			// will give us another.
			for _ in 0..2 * leaves.len() - 5 {
				llcl_boundary_pm(leaves, lists, nodes)?;
			}

			// Add the last chain and write the results!
			let node = Node::last(&lists[lists.len() - 2], &lists[lists.len() - 1], leaves);
			Self::llcl_write(node, leaves)
		}

		/// # Write Code Lengths!
		fn llcl_write(mut node: Node, leaves: &[Leaf<'_>]) -> Result<(), ZopfliError> {
			// Make sure we counted correctly before doing anything else.
			let mut last_count = node.count;
			debug_assert!(leaves.len() >= last_count.get() as usize);

			// Write the changes!
			let mut writer = leaves.iter().take(last_count.get() as usize).rev();
			for value in DeflateSym::LIMITED.into_iter().take(MAXBITS) {
				// Pull the next tail, if any.
				if let Some(tail) = node.tail.copied() {
					// Wait for a change in counts to write the values.
					if tail.count < last_count {
						for leaf in writer.by_ref().take((last_count.get() - tail.count.get()) as usize) {
							leaf.bitlength.set(value);
						}
						last_count = tail.count;
					}
					node = tail;
				}
				// Write the remaining entries and quit!
				else {
					for leaf in writer { leaf.bitlength.set(value); }
					return Ok(());
				}
			}

			// This shouldn't be reachable.
			Err(zopfli_error!())
		}
	}
}

/// # Length Limited Code Lengths.
///
/// This trait adds an `llcl` method to symbol count arrays that generates the
/// appropriate deflate symbols (bitlengths).
pub(crate) trait LengthLimitedCodeLengths<const MAXBITS: usize, const N: usize>: sealed::LengthLimitedCodeLengthsSealed<MAXBITS, N>
where Self: Sized {
	fn llcl(&self) -> Result<[DeflateSym; N], ZopfliError>;
	fn llcl_symbols(lengths: &[DeflateSym; N]) -> Result<Self, ZopfliError>;
}

impl sealed::LengthLimitedCodeLengthsSealed<7, 19> for [u32; 19] {}
impl sealed::LengthLimitedCodeLengthsSealed<15, ZOPFLI_NUM_D> for ArrayD<u32> {}
impl sealed::LengthLimitedCodeLengthsSealed<15, ZOPFLI_NUM_LL> for ArrayLL<u32> {}

macro_rules! llcl {
	($size:expr) => (
		/// # Counts to Symbols.
		fn llcl(&self) -> Result<[DeflateSym; $size], ZopfliError> {
			use sealed::LengthLimitedCodeLengthsSealed;

			// Start the bitlengths at zero.
			let mut bitlengths = [DeflateSym::D00; $size];
			let bitcells = array_of_cells(&mut bitlengths);

			// Crunch!
			SCRATCH.with(|nodes| Self::_llcl(self, bitcells, nodes)).map(|()| bitlengths)
		}
	);
}

macro_rules! llcl_symbols {
	($maxbits:literal, $size:expr) => (
		/// # Symbols to Counts.
		fn llcl_symbols(lengths: &[DeflateSym; $size]) -> Result<Self, ZopfliError> {
			// Count up the codes by code length.
			let mut counts: [u32; $maxbits] = [0; $maxbits];
			for l in lengths.iter().copied() {
				if (l as u8) < $maxbits { counts[l as usize] += 1; }
				else { return Err(zopfli_error!()); }
			}

			// Find the numerical value of the smallest code for each code length.
			counts[0] = 0;
			let mut code = 0;
			let mut next_code: [u32; $maxbits] = counts.map(|c| {
				let old_code = code;
				code = (code + c) << 1;
				old_code
			});

			// Update the symbols accordingly.
			let symbols: [u32; $size] = lengths.map(|l|
				if (1..$maxbits).contains(&(l as u8)) {
					let old_code = next_code[l as usize];
					next_code[l as usize] += 1;
					old_code
				}
				else { 0 }
			);

			Ok(symbols)
		}
	);
}

// Tree symbols have seven maxbits, while NUM_D and NUM_LL each have 15.
impl LengthLimitedCodeLengths<7, 19> for [u32; 19] {
	llcl!(19);
	llcl_symbols!(8, 19);
}

impl LengthLimitedCodeLengths<15, ZOPFLI_NUM_D> for ArrayD<u32> {
	llcl!(ZOPFLI_NUM_D);
	llcl_symbols!(16, ZOPFLI_NUM_D);
}

impl LengthLimitedCodeLengths<15, ZOPFLI_NUM_LL> for ArrayLL<u32> {
	llcl!(ZOPFLI_NUM_LL);
	/// # Symbols to Counts.
	fn llcl_symbols(lengths: &ArrayLL<DeflateSym>) -> Result<Self, ZopfliError> {
		// Count up the codes by code length.
		let mut counts: [u32; 16] = [0; 16];
		for l in lengths.iter().copied() {
			if (l as u8) < 16 { counts[l as usize] += 1; }
			else { return Err(zopfli_error!()); }
		}

		// Find the numerical value of the smallest code for each code length.
		counts[0] = 0;
		let mut code = 0;
		let mut next_code: [u32; 16] = counts.map(|c| {
			let old_code = code;
			code = (code + c) << 1;
			old_code
		});

		// Update the symbols accordingly.
		let mut symbols = [0; ZOPFLI_NUM_LL];
		for (s, l) in symbols.iter_mut().zip(lengths.iter().copied()) {
			if (1..16).contains(&(l as u8)) {
				*s = next_code[l as usize];
				next_code[l as usize] += 1;
			}
		}

		Ok(symbols)
	}
}



/// # Tree Lengths and Distances.
///
/// This struct is used for calculating the optimal DEFLATE tree size and/or
/// writing the tree data to the output.
///
/// Both involve doing (virtually) the same thing several times in a row, so
/// the centralized storage helps reduce a little bit of that overhead.
pub(crate) struct TreeLd<'a> {
	ll_lengths: &'a ArrayLL<DeflateSym>,
	d_lengths: &'a ArrayD<DeflateSym>,
	hlit: usize,
	hdist: usize,
}

impl<'a> TreeLd<'a> {
	/// # New.
	const fn new(
		ll_lengths: &'a ArrayLL<DeflateSym>,
		d_lengths: &'a ArrayD<DeflateSym>,
	) -> Self {
		// Find the last non-zero length symbol, starting from 285. (The offset
		// marks the boundary between literals and symbols; we'll use both in
		// some places, and not in others.)
		let mut hlit = 29;
		while hlit > 0 && ll_lengths[256 + hlit].is_zero() { hlit -= 1; }

		// Now the same for distance, starting at 29 proper.
		let mut hdist = 29;
		while hdist > 0 && d_lengths[hdist].is_zero() { hdist -= 1; }

		Self {
			ll_lengths,
			d_lengths,
			hlit,
			hdist,
		}
	}

	/// # Total Entries.
	///
	/// Return the combined length and distance lengths that can be
	/// traversed.
	const fn len(&self) -> usize { self.hlit + 257 + self.hdist + 1 }

	#[allow(unsafe_code)]
	/// # Symbol.
	///
	/// Crunching loops through the length and distance symbols as if they were
	/// one contiguous set. This returns the appropriate symbol given the
	/// index. (Length symbols are returned first; once we run out of those
	/// distance symbols are returned instead.)
	///
	/// The compiler doesn't really understand what we're doing, hence the
	/// unsafe. (Also of note, all raw `u32` are checked during `TreeLd::new`
	/// to ensure they can be represented as `DeflateSym`.)
	fn symbol(&self, idx: usize) -> DeflateSym {
		let ll_len = self.hlit + 257;

		// Fetch it from the lengths table.
		if idx < ll_len {
			unsafe { *self.ll_lengths.get_unchecked(idx) }
		}
		// Fetch it from the distance table.
		else {
			debug_assert!(idx - ll_len <= 29);
			unsafe { *self.d_lengths.get_unchecked(idx - ll_len) }
		}
	}
}

impl<'a> TreeLd<'a> {
	/// # Calculate the Exact Tree Size (in Bits).
	///
	/// This returns the index (0..8) that produced the smallest size, along
	/// with that size.
	pub(crate) fn calculate_tree_size(
		ll_lengths: &'a ArrayLL<DeflateSym>,
		d_lengths: &'a ArrayD<DeflateSym>,
	) -> Result<(u8, u32), ZopfliError> {
		let tree = Self::new(ll_lengths, d_lengths);
		let mut best_size = u32::MAX;
		let mut best_idx = 0;

		// Try every combination.
		for idx in 0..8 {
			let size = tree.crunch(idx, None)?;
			if size < best_size {
				best_size = size;
				best_idx = idx;
			}
		}

		Ok((best_idx, best_size))
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Encode Tree.
	///
	/// This finds the index that produces the smallest tree size, then writes
	/// that table's bits to the output.
	pub(crate) fn encode_tree(
		ll_lengths: &'a ArrayLL<DeflateSym>,
		d_lengths: &'a ArrayD<DeflateSym>,
		extra: u8,
		out: &mut ZopfliOut,
	) -> Result<(), ZopfliError> {
		let tree = Self::new(ll_lengths, d_lengths);
		tree.crunch(extra, Some(out))?;
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Crunch the Tree.
	///
	/// This crunches the data for the given index, either returning the size
	/// or writing it to the output (and returning zero).
	fn crunch(&self, extra: u8, out: Option<&mut ZopfliOut>) -> Result<u32, ZopfliError> {
		// Are we using any of the special alphabet parts?
		let use_16 = 0 != extra & 1;
		let use_17 = 0 != extra & 2;
		let use_18 = 0 != extra & 4;

		// We need a structure to hold the counts for each symbol.
		let mut cl_counts = [0_u32; 19];

		// We also need a structure to hold the positional symbol/count data,
		// but only if we're going to write the tree at the end. If not, this
		// will remain unallocated (and hopefully have minimal runtime impact).
		let mut rle = Vec::new();
		if out.is_some() { rle.reserve(self.len()); }

		// Run through all the length symbols, then the distance symbols, with
		// the odd skip to keep us on our toes.
		let mut i = 0;
		while i < self.len() {
			let mut count: u32 = 1;
			let symbol = self.symbol(i);

			// Peek ahead; we may be able to do more in one go.
			if use_16 || (symbol.is_zero() && (use_17 || use_18)) {
				let mut j = i + 1;
				while j < self.len() && symbol == self.symbol(j) {
					count += 1;
					j += 1;
				}

				// Skip these indices, if any, on the next pass.
				i += (count - 1) as usize;
			}

			// Repetitions of zeroes.
			if symbol.is_zero() && count >= 3 {
				if use_18 {
					while count >= 11 {
						let count2 = count.min(138);
						if out.is_some() {
							rle.push((DeflateSym::D18, count2 - 11));
						}
						cl_counts[DeflateSym::D18 as usize] += 1;
						count -= count2;
					}
				}
				if use_17 {
					while count >= 3 {
						let count2 = count.min(10);
						if out.is_some() {
							rle.push((DeflateSym::D17, count2 - 3));
						}
						cl_counts[DeflateSym::D17 as usize] += 1;
						count -= count2;
					}
				}
			}

			// Repetitions of any symbol.
			if use_16 && count >= 4 {
				// The first one always counts.
				count -= 1;
				if out.is_some() { rle.push((symbol, 0)); }
				cl_counts[symbol as usize] += 1;

				while count >= 3 {
					let count2 = count.min(6);
					if out.is_some() {
						rle.push((DeflateSym::D16, count2 - 3));
					}
					cl_counts[DeflateSym::D16 as usize] += 1;
					count -= count2;
				}
			}

			// Deal with non- or insufficiently-repeating values.
			if out.is_some() {
				for _ in 0..count { rle.push((symbol, 0)); }
			}
			cl_counts[symbol as usize] += count;
			i += 1;
		}

		// Update the lengths and symbols given the counts.
		let cl_lengths = cl_counts.llcl()?;

		// Find the last non-zero count.
		let mut hclen = 15;
		while hclen > 0 && cl_counts[DeflateSym::TREE[hclen + 3] as usize] == 0 {
			hclen -= 1;
		}

		#[allow(clippy::option_if_let_else)]
		// Write the results?
		if let Some(out) = out {
			self.crunch_write(cl_lengths, out, hclen, rle)
		}
		// Just calculate the would-be size and return.
		else {
			Ok(Self::crunch_size(cl_lengths, cl_counts, hclen))
		}
	}

	#[allow(clippy::cast_possible_truncation)]
	fn crunch_size(
		cl_lengths: [DeflateSym; 19],
		cl_counts: [u32; 19],
		hclen: usize,
	) -> u32 {
		let mut size = 14;
		size += (hclen as u32 + 4) * 3;
		for (a, b) in cl_lengths.iter().copied().zip(cl_counts.iter().copied()) {
			size += (a as u32) * b;
		}
		size += cl_counts[16] * 2; // Extra bits.
		size += cl_counts[17] * 3;
		size +  cl_counts[18] * 7
	}

	#[allow(clippy::cast_possible_truncation)]
	fn crunch_write(
		&self,
		cl_lengths: [DeflateSym; 19],
		out: &mut ZopfliOut,
		hclen: usize,
		rle: Vec<(DeflateSym, u32)>,
	) -> Result<u32, ZopfliError> {
		// Convert the lengths to symbols.
		let cl_symbols = <[u32; 19]>::llcl_symbols(&cl_lengths)?;

		// Write the main lengths.
		out.add_bits(self.hlit as u32, 5);
		out.add_bits(self.hdist as u32, 5);
		out.add_bits(hclen as u32, 4);

		// Write each cl_length in the jumbled DEFLATE order.
		for &o in &DeflateSym::TREE[..hclen + 4] {
			out.add_bits(cl_lengths[o as usize] as u32, 3);
		}

		// Write each symbol in order of appearance along with its extra bits,
		// if any.
		for (a, b) in rle {
			let symbol = cl_symbols[a as usize];
			out.add_huffman_bits(symbol, cl_lengths[a as usize] as u32);

			// Extra bits.
			match a {
				DeflateSym::D16 => { out.add_bits(b, 2); },
				DeflateSym::D17 => { out.add_bits(b, 3); },
				DeflateSym::D18 => { out.add_bits(b, 7); },
				_ => {},
			}
		}

		// We have to return a number, so why not zero?
		Ok(0)
	}
}



/// # Node Scratch.
///
/// This is a super-cheap arena-like structure for holding all the temporary
/// data required for length-limited-code-length calculations.
struct KatScratch {
	leaves: NonNull<u8>,
	lists: NonNull<u8>,
	nodes: NonNull<u8>,
	nodes_len: Cell<usize>,
}

impl Drop for KatScratch {
	#[allow(unsafe_code)]
	/// # Drop.
	///
	/// We might as well free the memory associated with the backing arrays
	/// before we go.
	fn drop(&mut self) {
		unsafe {
			std::alloc::dealloc(self.leaves.as_ptr(), Self::LEAVES_LAYOUT);
			std::alloc::dealloc(self.lists.as_ptr(), Self::LIST_LAYOUT);
			std::alloc::dealloc(self.nodes.as_ptr(), Self::NODE_LAYOUT);
		}
	}
}

impl KatScratch {
	/// # Max Nodes.
	///
	/// This represents the theoretical maximum number of nodes a length-
	/// limiting pass might generate.
	const MAX: usize = (2 * ZOPFLI_NUM_LL - 2) * 15;

	/// # Leaves Array Layout.
	const LEAVES_LAYOUT: Layout = Layout::new::<[Leaf<'_>; ZOPFLI_NUM_LL]>();

	/// # List Array Layout.
	const LIST_LAYOUT: Layout = Layout::new::<[List; 15]>();

	/// # Node Array Layout.
	const NODE_LAYOUT: Layout = Layout::new::<[Node; Self::MAX]>();

	#[allow(unsafe_code)]
	/// # New!
	///
	/// Return a new instance of self, allocated but uninitialized.
	///
	/// Similar to other mega-array structures like `ZopfliHash`, its members
	/// are manually allocated from pointers to keep them off the stack. Unlike
	/// the others, though, the `KatScratch` members remain in pointer form to
	/// prevent lifetime/borrow-checker confusion.
	fn new() -> Self {
		let leaves: NonNull<u8> = NonNull::new(unsafe { alloc(Self::LEAVES_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::LEAVES_LAYOUT));

		let lists: NonNull<u8> = NonNull::new(unsafe { alloc(Self::LIST_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::LIST_LAYOUT));

		let nodes: NonNull<u8> = NonNull::new(unsafe { alloc(Self::NODE_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::NODE_LAYOUT));

		Self {
			leaves,
			lists,
			nodes,
			nodes_len: Cell::new(0),
		}
	}

	#[allow(unsafe_code)]
	/// # Make Leaves.
	///
	/// Join the non-zero frequencies with their corresponding bitlengths into
	/// a collection of leaves, then return it sorted.
	///
	/// ## Safety
	///
	/// The returned reference remains valid for the duration of the length-
	/// limited method call because:
	/// 1. The values are written only once (here);
	/// 2. The backing storage is not reallocated;
	/// 3. Values leftover from prior passes are never reread;
	///
	/// `Leaf` is `Copy` so there's nothing to drop, per se, but the overall
	/// memory associated with the backing array will get deallocated as part
	/// of `Self::drop`.
	fn leaves<'a, const N: usize>(
		&self,
		frequencies: &'a [u32; N],
		bitlengths: &'a [Cell<DeflateSym>; N],
	) -> &[Leaf<'a>] {
		let mut len = 0;
		let ptr = self.leaves.cast::<Leaf<'_>>().as_ptr();
		for (frequency, bitlength) in frequencies.iter().copied().zip(bitlengths) {
			if let Some(frequency) = NonZeroU32::new(frequency) {
				unsafe {
					// Safety: the maximum N is ZOPFLI_NUM_LL, so this will
					// always be in range.
					ptr.add(len).write(Leaf { frequency, bitlength });
				}
				len += 1;
			}
		}

		// Safety: by writing before reading, we know this portion is
		// initialized.
		let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };
		slice.sort();
		slice
	}

	#[allow(unsafe_code)]
	/// # Starter List.
	///
	/// This resets the internal node count, adds two new starter nodes, then
	/// returns a `List` referencing them.
	///
	/// See `Self::push` for details about the internal `Node` storage and
	/// safety details.
	unsafe fn init_list(&self, weight1: NonZeroU32, weight2: NonZeroU32) -> List {
		// Reset the length counter to two for the two nodes we're about to
		// create.
		self.nodes_len.set(2);

		// The first node.
		let ptr = self.nodes.cast::<Node>().as_ptr();
		ptr.write(Node {
			weight: weight1,
			count: NZ1,
			tail: None,
		});
		let lookahead0 = &*ptr;

		// The second node.
		let ptr = ptr.add(1);
		ptr.write(Node {
			weight: weight2,
			count: NZ2,
			tail: None,
		});
		let lookahead1 = &*ptr;

		// And finally the list!
		List { lookahead0, lookahead1 }
	}

	#[allow(unsafe_code)]
	/// # Make Lists.
	///
	/// This resets the internal node counts and returns a slice of `len`
	/// starter lists for the calculations to work from.
	///
	/// ## Safety
	///
	/// The returned reference remains valid for the duration of the length-
	/// limited method call because:
	/// 1. The pointer is only directly accessed once (here);
	/// 2. The backing storage is not reallocated;
	/// 3. Values leftover from prior passes are never reread;
	///
	/// `List` is `Copy` so there's nothing to drop, per se, but the overall
	/// memory associated with the backing array will get deallocated as part
	/// of `Self::drop`.
	fn lists(&self, len: usize, weight1: NonZeroU32, weight2: NonZeroU32)
	-> &'static mut [List] {
		// Fifteen is the max MAXBITS used by the program so length will never
		// actually be out of range, but there's no harm in verifying that
		// during debug runs.
		debug_assert!(len <= 15);

		// Create a `List` with two starting `Node`s, then copy it `len` times
		// into our backing array.
		unsafe {
			// Safety: we verified the length is in range, and since we're
			// writing before reading, the return value will have been
			// initialized.
			let list = self.init_list(weight1, weight2);
			let ptr = self.lists.cast::<List>().as_ptr();
			for i in 0..len {
				ptr.add(i).write(list);
			}

			// Return the slice corresponding to the values we just wrote!
			std::slice::from_raw_parts_mut(ptr, len)
		}
	}

	#[allow(unsafe_code)]
	/// # Push.
	///
	/// Push a new node to the store and return an immutable reference to it.
	///
	/// This method is technically fallible, but should never actually fail as
	/// the backing storage is pre-allocated to the theoretical maximum, but
	/// given all the `unsafe`, it feels better to verify that at runtime.
	///
	/// ## Safety:
	///
	/// The returned reference remains valid for the duration of the length-
	/// limited method call because:
	/// 1. The values are written only once (here);
	/// 2. The backing storage is not reallocated;
	/// 3. Values leftover from prior passes are never reread;
	///
	/// `Node` is `Copy` so there's nothing to drop, per se, but the overall
	/// memory associated with the backing array will get deallocated as part
	/// of `Self::drop`.
	fn push(&self, node: Node) -> Result<&'static Node, ZopfliError> {
		// Pull the current length and verify we have room to add more nodes.
		// This should never not be true, but zopfli is confusing so caution is
		// appropriate. Haha.
		let len = self.nodes_len.get();
		if len < Self::MAX {
			// Increment the length for next time.
			self.nodes_len.set(len + 1);

			unsafe {
				// Safety: we just verified the length is in range, and because
				// we're writing before reading, we know our return will have
				// been initialized.
				let ptr = self.nodes.cast::<Node>().as_ptr().add(len);
				ptr.write(node); // Copy the value into position.
				Ok(&*ptr)        // Return a reference to it.
			}
		}
		// If we somehow surpassed the theoretical maximum, return an error to
		// abort further processing of this image.
		else { Err(zopfli_error!()) }
	}
}



#[derive(Clone, Copy)]
/// # Leaf.
///
/// This is a simple tuple containing a non-zero frequency and its companion
/// bitlength.
struct Leaf<'a> {
	frequency: NonZeroU32,
	bitlength: &'a Cell<DeflateSym>,
}

impl<'a> Eq for Leaf<'a> {}

impl<'a> Ord for Leaf<'a> {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { self.frequency.cmp(&other.frequency) }
}

impl<'a> PartialEq for Leaf<'a> {
	#[inline]
	fn eq(&self, other: &Self) -> bool { self.frequency == other.frequency }
}

impl<'a> PartialOrd for Leaf<'a> {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}



#[derive(Clone, Copy)]
/// # List.
///
/// This struct holds a pair of recursive node chains.
struct List {
	lookahead0: &'static Node,
	lookahead1: &'static Node,
}

impl List {
	/// # Rotate.
	fn rotate(&mut self) { self.lookahead0 = self.lookahead1; }

	/// # Weight Sum.
	const fn weight_sum(&self) -> NonZeroU32 {
		self.lookahead0.weight.saturating_add(self.lookahead1.weight.get())
	}
}



#[derive(Clone, Copy)]
/// # Node.
struct Node {
	weight: NonZeroU32,
	count: NonZeroU32,
	tail: Option<&'static Node>,
}

impl Node {
	/// # Finish Last Node!
	///
	/// This method establishes the final tail that the subsequent writing
	/// will start with.
	fn last(list_y: &List, list_z: &List, leaves: &[Leaf<'_>]) -> Self {
		// Figure out the final node!
		let last_count = list_z.lookahead1.count;
		let weight_sum = list_y.weight_sum();
		if (last_count.get() as usize) < leaves.len() && leaves[last_count.get() as usize].frequency < weight_sum {
			Self {
				weight: NZ1, // We'll never look at this value.
				count: last_count.saturating_add(1),
				tail: list_z.lookahead1.tail,
			}
		}
		else {
			Self {
				weight: NZ1, // We'll never look at this value.
				count: last_count,
				tail: Some(list_y.lookahead1),
			}
		}
	}
}



#[allow(unsafe_code)]
/// Array of Cells.
///
/// Revisualize a mutable array as an array of cells.
///
/// TODO: use `Cell::as_array_of_cells` once stabilized.
fn array_of_cells<T, const N: usize>(arr: &mut [T; N]) -> &[Cell<T>; N] {
	let cells = Cell::from_mut(arr);
	// Safety: `Cell<T>` has the same memory layout as `T`.
	unsafe { &*(std::ptr::from_ref(cells).cast::<[Cell<T>; N]>()) }
}

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or combination of
/// two chains from the previous list.
fn llcl_boundary_pm(leaves: &[Leaf<'_>], lists: &mut [List], nodes: &KatScratch)
-> Result<(), ZopfliError> {
	// This method should never be called with an empty list.
	let [rest @ .., current] = lists else { return Err(zopfli_error!()); };
	let last_count = current.lookahead1.count;

	// We're at the beginning, which is the end since we're iterating in
	// reverse.
	if rest.is_empty() {
		if let Some(last_leaf) = leaves.get(last_count.get() as usize) {
			// Shift the lookahead and add a new node.
			current.rotate();
			current.lookahead1 = nodes.push(Node {
				weight: last_leaf.frequency,
				count: last_count.saturating_add(1),
				tail: current.lookahead0.tail,
			})?;
		}
		return Ok(());
	}

	// Shift the lookahead.
	current.rotate();

	let previous = rest[rest.len() - 1];
	let weight_sum = previous.weight_sum();

	// Add a leaf and increment the count.
	if let Some(last_leaf) = leaves.get(last_count.get() as usize) {
		if last_leaf.frequency < weight_sum {
			current.lookahead1 = nodes.push(Node {
				weight: last_leaf.frequency,
				count: last_count.saturating_add(1),
				tail: current.lookahead0.tail,
			})?;
			return Ok(());
		}
	}

	// Update the tail.
	current.lookahead1 = nodes.push(Node {
		weight: weight_sum,
		count: last_count,
		tail: Some(previous.lookahead1),
	})?;

	// Replace the used-up lookahead chains by recursing twice.
	llcl_boundary_pm(leaves, rest, nodes)?;
	llcl_boundary_pm(leaves, rest, nodes)
}



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// # Test Maxbits.
	///
	/// The original zopfli code included a check to ensure the MAXBITS were
	/// sufficient for the number of leaves. The ceilings are all hard-coded so
	/// there's no reason to look for that at runtime, but let's triple-check
	/// ourselves here!
	fn t_maxbits() {
		for (maxbits, size) in [(7, 19), (15, ZOPFLI_NUM_D), (15, ZOPFLI_NUM_LL)] {
			assert!(size < (1 << maxbits));
		}
	}

	// The following tests have been adapted from the zopfli-rs crate:
	// <https://github.com/zopfli-rs/zopfli/blob/main/src/katajainen.rs>

	#[test]
	fn t_kat7() {
		let f = [252, 0, 1, 6, 9, 10, 6, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
		assert_eq!(
			f.llcl(),
			Ok([
				DeflateSym::D01, DeflateSym::D00, DeflateSym::D06, DeflateSym::D04,
				DeflateSym::D03, DeflateSym::D03, DeflateSym::D03, DeflateSym::D05,
				DeflateSym::D06, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00
			]),
		);
	}

	#[test]
	fn t_kat15() {
		let f = [
			0, 0, 0, 0, 0, 0, 18, 0, 6, 0, 12, 2, 14, 9, 27, 15,
			23, 15, 17, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		];
		assert_eq!(
			f.llcl(),
			Ok([
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D03, DeflateSym::D00,
				DeflateSym::D05, DeflateSym::D00, DeflateSym::D04, DeflateSym::D06,
				DeflateSym::D04, DeflateSym::D04, DeflateSym::D03, DeflateSym::D04,
				DeflateSym::D03, DeflateSym::D03, DeflateSym::D03, DeflateSym::D04,
				DeflateSym::D06, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
			])
		);
	}

	#[test]
	fn t_kat_limited() {
		// No frequencies.
		let mut f = [0; 19];
		assert_eq!(
			f.llcl(),
			Ok([DeflateSym::D00; 19]),
		);

		// One frequency.
		f[2] = 10;
		assert_eq!(
			f.llcl(),
			Ok([
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D01, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00
			]),
		);

		// Two frequencies.
		f[0] = 248;
		assert_eq!(
			f.llcl(),
			Ok([
				DeflateSym::D01, DeflateSym::D00, DeflateSym::D01, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00
			]),
		);
	}
}
