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
	num::{
		NonZeroU32,
		NonZeroUsize,
	},
	ptr::NonNull,
};
use super::{
	ArrayD,
	ArrayLL,
	DeflateSym,
	DeflateSymBasic,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
	ZopfliOut,
};



#[allow(unsafe_code)]
/// # One is Non-Zero.
const NZ01: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(1) };

#[allow(unsafe_code)]
/// # Two is Non-Zero.
const NZ02: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };

#[allow(unsafe_code)]
/// # Fourteen is Non-Zero.
const NZ14: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(14) };

/// # Zero-Filled Tree Counts.
const ZEROED_COUNTS_TREE: [u32; 19] = [0; 19];



thread_local!(
	/// # Shared Node Scratch.
	///
	/// The length-limited-code-length methods need to temporarily store
	/// thousands of `Node` objects. Using a thread-local share for that cuts
	/// way down on the number of allocations we'd otherwise have to make!
	static KATSCRATCH: KatScratch = KatScratch::new()
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
	pub trait LengthLimitedCodeLengthsSealed<const N: usize> {
		const MAXBITS: NonZeroUsize;

		#[inline]
		/// # Crunch the Code Lengths.
		///
		/// This method serves as the closure for the caller's call to
		/// `KATSCRATCH.with_borrow_mut()`. It does all that needs doing to get
		/// the desired length-limited data into the provided `bitlengths`.
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
				usize::min(Self::MAXBITS.get(), leaves.len() - 1),
				leaves[0].frequency,
				leaves[1].frequency,
			);
			// Safety: `usize::min(MAXBITS, leaves.len() - 1)` (above) is
			// how many lists we'll have, and since MAXBITS is at least
			// seven and leaves.len() at least three, we'll always have at
			// least two lists to work with.
			if lists.len() < 2 { crate::unreachable(); }

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

		#[inline]
		/// # Write Code Lengths!
		///
		/// This is the final stage of the LLCL chain, where the results are
		/// finally recorded!
		fn llcl_write(mut node: Node, leaves: &[Leaf<'_>]) -> Result<(), ZopfliError> {
			// Make sure we counted correctly before doing anything else.
			let mut last_count = node.count;
			debug_assert!(leaves.len() >= last_count.get() as usize);

			// Write the changes!
			let mut writer = leaves.iter().take(last_count.get() as usize).rev();
			for value in DeflateSym::nonzero_iter().take(Self::MAXBITS.get()) {
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
pub(crate) trait LengthLimitedCodeLengths<const N: usize>: sealed::LengthLimitedCodeLengthsSealed<N>
where Self: Sized {
	fn llcl(&self) -> Result<[DeflateSym; N], ZopfliError>;
	fn llcl_symbols(lengths: &[DeflateSym; N]) -> Self;
}

impl sealed::LengthLimitedCodeLengthsSealed<19> for [u32; 19] {
	#[allow(unsafe_code)]
	const MAXBITS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(7) };
}
impl sealed::LengthLimitedCodeLengthsSealed<ZOPFLI_NUM_D> for ArrayD<u32> {
	#[allow(unsafe_code)]
	const MAXBITS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(15) };
}
impl sealed::LengthLimitedCodeLengthsSealed<ZOPFLI_NUM_LL> for ArrayLL<u32> {
	#[allow(unsafe_code)]
	const MAXBITS: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(15) };
}

macro_rules! llcl {
	($maxbits:literal, $size:expr) => (
		/// # Counts to Symbols.
		fn llcl(&self) -> Result<[DeflateSym; $size], ZopfliError> {
			use sealed::LengthLimitedCodeLengthsSealed;

			// Start the bitlengths at zero.
			let mut bitlengths = [DeflateSym::D00; $size];
			let bitcells = array_of_cells(&mut bitlengths);

			// Crunch!
			KATSCRATCH.with(|nodes| Self::_llcl(self, bitcells, nodes)).map(|()| bitlengths)
		}

		#[inline]
		/// # Symbols to Counts.
		fn llcl_symbols(lengths: &[DeflateSym; $size]) -> Self {
			// The lengths should have previously been limited.
			debug_assert!(lengths.iter().all(|&l| (l as usize) < $maxbits));

			// Count up the codes by code length. (Note: the compiler doesn't
			// understand the lengths have been limited to $maxbits. Of all the
			// different ways to get it to elide bounds checks, overallocating
			// scratch to 19 performs best.
			let mut scratch = ZEROED_COUNTS_TREE;
			for l in lengths.iter().copied() { scratch[l as usize] += 1; }

			// Find the numerical value of the smallest code for each code
			// length (up to $maxbits).
			let mut code = 0;
			scratch[0] = 0;
			for c in scratch.iter_mut().take($maxbits) {
				let next = (code + *c) << 1;
				*c = std::mem::replace(&mut code, next);
			}

			// Update the (non-zero) symbol counts accordingly.
			let mut symbols = [0; $size];
			for (l, s) in lengths.iter().copied().zip(&mut symbols) {
				if ! l.is_zero() {
					*s = scratch[l as usize];
					scratch[l as usize] += 1;
				}
			}
			symbols
		}
	);
}

// Tree symbols have seven maxbits, while NUM_D and NUM_LL each have 15.
impl LengthLimitedCodeLengths<19> for [u32; 19] {
	llcl!(8, 19);
}

impl LengthLimitedCodeLengths<ZOPFLI_NUM_D> for ArrayD<u32> {
	llcl!(16, ZOPFLI_NUM_D);
}

impl LengthLimitedCodeLengths<ZOPFLI_NUM_LL> for ArrayLL<u32> {
	llcl!(16, ZOPFLI_NUM_LL);
}



/// # Calculate the Exact Tree Size (in Bits).
///
/// This returns the index (0..8) that produced the smallest size, along
/// with that size.
pub(crate) fn best_tree_size(
	ll_lengths: &ArrayLL<DeflateSym>,
	d_lengths: &ArrayD<DeflateSym>,
) -> Result<(u8, NonZeroU32), ZopfliError> {
	// Drop the last two zeroes plus any trailing zeroes, then merge them
	// together into a single collection.
	let all: Vec<DeflateSym> = {
		let mut ll_lengths = &ll_lengths[..286];
		while let [rest @ .., DeflateSym::D00] = ll_lengths {
			ll_lengths = rest;
			if ll_lengths.len() == 257 { break; } // Keep all literals.
		}

		let mut d_lengths = &d_lengths[..30];
		while let [rest @ .., DeflateSym::D00] = d_lengths { d_lengths = rest; }

		[ll_lengths, d_lengths].concat()
	};

	// Our targets!
	let mut best_extra = 0;
	let mut best_size = NonZeroU32::MAX;

	for extra in 0..8 {
		let cl_counts = best_tree_size_counts(&all, extra);
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
	// Drop the last two zeroes plus any trailing zeroes, then merge them
	// together into a single collection.
	let mut hlit: u32 = 29;
	let mut hdist: u32 = 29;
	let all: Vec<DeflateSym> = {
		let mut ll_lengths = &ll_lengths[..286];
		while let [rest @ .., DeflateSym::D00] = ll_lengths {
			ll_lengths = rest;
			hlit -= 1;
			if ll_lengths.len() == 257 { break; } // Keep all literals.
		}

		let mut d_lengths = &d_lengths[..30];
		while let [rest @ .., DeflateSym::D00] = d_lengths {
			d_lengths = rest;
			hdist -= 1;
		}

		[ll_lengths, d_lengths].concat()
	};

	// We'll need to store some RLE symbols and positions too.
	let mut rle: Vec<(DeflateSym, u16)> = Vec::new();

	let cl_counts = encode_tree_counts(&all, &mut rle, extra);
	let cl_lengths = cl_counts.llcl()?;
	let hclen = tree_hclen(&cl_counts);
	let cl_symbols = <[u32; 19]>::llcl_symbols(&cl_lengths);

	// Write the main lengths.
	out.add_fixed_bits::<5>(hlit);
	out.add_fixed_bits::<5>(hdist);
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



/// # Node Scratch.
///
/// This is a super-cheap arena-like structure for holding all the temporary
/// data required for length-limited-code-length calculations.
///
/// This requires doing some fairly un-Rust-like things, but that would be
/// equally true of any third-party structure as well, and since we know the
/// particulars in advance, we can do it leaner and meaner ourselves.
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
	/// limiting pass might generate, though it is unlikely to ever be reached
	/// in practice. (Better safe than sorry!)
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
	/// Return a new instance of self, allocated but **uninitialized**.
	///
	/// Similar to other mega-array structures like `ZopfliHash`, its members
	/// are manually allocated from pointers to keep them off the stack. Unlike
	/// the others, though, the `KatScratch` members remain in pointer form to
	/// prevent subsequent lifetime/borrow-checker confusion.
	///
	/// ## Safety
	///
	/// New values are written from pointers without first reading or dropping
	/// the previous values at that position, and references to the new values
	/// are only made available after said write, eliminating any UB weirdness
	/// from possibly-uninitialized data.
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
	#[inline]
	/// # Make Leaves.
	///
	/// Join the non-zero frequencies with their corresponding bitlengths into
	/// a collection of leaves. That collection is then sorted and returned.
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
		const {
			// Abort with a compilation error if for some reason we try to
			// pass more leaves than we've got room for.
			assert!(N <= ZOPFLI_NUM_LL);
		}

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
			count: NZ01,
			tail: None,
		});
		let lookahead0 = &*ptr;

		// The second node.
		let ptr = ptr.add(1);
		ptr.write(Node {
			weight: weight2,
			count: NZ02,
			tail: None,
		});
		let lookahead1 = &*ptr;

		// And finally the list!
		List { lookahead0, lookahead1 }
	}

	#[allow(unsafe_code)]
	#[inline]
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
	#[inline]
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
/// This struct holds a pair of recursive node chains. The lifetimes are
/// technically static, but in practice are always scoped to the more limited
/// lifetime of the borrow. (`List`s are never accessible once the session that
/// birthed them has closed.)
struct List {
	lookahead0: &'static Node,
	lookahead1: &'static Node,
}

impl List {
	#[inline]
	/// # Rotate.
	///
	/// Replace the first chain with a copy of the second.
	fn rotate(&mut self) { self.lookahead0 = self.lookahead1; }

	#[inline]
	/// # Weight Sum.
	///
	/// Add and return the sum of the weights of the two chains.
	const fn weight_sum(&self) -> NonZeroU32 {
		self.lookahead0.weight.saturating_add(self.lookahead1.weight.get())
	}
}



#[derive(Clone, Copy)]
/// # Node.
///
/// This holds a weight and frequency pair, and possibly a reference to the
/// previous `Node` this one replaced.
///
/// As with `List`, the static lifetime is technically true, but in practice
/// references will never extend beyond the current borrow.
struct Node {
	weight: NonZeroU32,
	count: NonZeroU32,
	tail: Option<&'static Node>,
}

impl Node {
	#[inline]
	/// # Finish Last Node!
	///
	/// This method creates and returns the final tail to be used as the
	/// starting point for the subsequent `llcl_write` call.
	fn last(list_y: &List, list_z: &List, leaves: &[Leaf<'_>]) -> Self {
		// Figure out the final node!
		let last_count = list_z.lookahead1.count;
		let weight_sum = list_y.weight_sum();
		if (last_count.get() as usize) < leaves.len() && leaves[last_count.get() as usize].frequency < weight_sum {
			Self {
				weight: NZ01, // We'll never look at this value.
				count: last_count.saturating_add(1),
				tail: list_z.lookahead1.tail,
			}
		}
		else {
			Self {
				weight: NZ01, // We'll never look at this value.
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
/// TODO: use `Cell::as_array_of_cells` once that method is stabilized.
fn array_of_cells<T, const N: usize>(arr: &mut [T; N]) -> &[Cell<T>; N] {
	let cells = Cell::from_mut(arr);
	// Safety: `Cell<T>` has the same memory layout as `T`.
	unsafe { &*(std::ptr::from_ref(cells).cast::<[Cell<T>; N]>()) }
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
		if use_16 || ((use_17 || use_18) && symbol.is_zero()) {
			let mut j = i + 1;
			while j < all.len() && symbol == all[j] {
				count += 1;
				j += 1;
				i += 1;
			}
		}

		// Repetitions of zeroes.
		if symbol.is_zero() && count >= 3 {
			if use_18 {
				special!(11, 138, D18);
			}
			if use_17 {
				special!(3, 10, D17);
			}
		}

		// Other symbol repetitions.
		if use_16 && count >= 4 {
			// Always count the first one as itself.
			count -= 1;
			cl_counts[symbol as usize] += 1;

			special!(3, 6, D16);
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
		if use_16 || ((use_17 || use_18) && symbol.is_zero()) {
			let mut j = i + 1;
			while j < all.len() && symbol == all[j] {
				count += 1;
				j += 1;
				i += 1;
			}
		}

		// Repetitions of zeroes.
		if count >= 3 && symbol.is_zero() {
			if use_18 {
				special!(11, 138, D18);
			}
			if use_17 {
				special!(3, 10, D17);
			}
		}

		// Other symbol repetitions.
		if use_16 && count >= 4 {
			// Always count the first one as itself.
			count -= 1;
			rle.push((symbol, 0));
			cl_counts[symbol as usize] += 1;

			special!(3, 6, D16);
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

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or combination of
/// two chains from the previous list.
///
/// Note: it would probably be more appropriate to make this a trait member or
/// at least scope it to the sealed trait's module, but doing either leads the
/// compiler to change its inlining decisions for the worse, so best to leave
/// it where it is!
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
	#[allow(unsafe_code)]
	// Safety: DeflateSymBasic covers all values between 0..=15.
	unsafe { std::mem::transmute::<u8, DeflateSymBasic>(hclen as u8) }
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

	#[test]
	/// # Tree Max.
	///
	/// Make sure our math correctly aligns with `TreeRleIdx`.
	fn t_tree_max() {
		assert_eq!(TreeScratch::MAX - 1, TreeRleIdx::T315 as usize);
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
