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
	DeflateSymBasic,
	TreeDist,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
	ZopfliOut,
};



#[expect(unsafe_code, reason = "Two is non-zero.")]
/// # Two is Non-Zero.
const NZ02: NonZeroU32 = unsafe { NonZeroU32::new_unchecked(2) };

#[expect(unsafe_code, reason = "Fourteen is non-zero.")]
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



/// # Length Limited Code Lengths.
///
/// This trait adds an `llcl` method to symbol count arrays that generates the
/// appropriate deflate symbols (bitlengths).
pub(crate) trait LengthLimitedCodeLengths<const N: usize>
where Self: Sized {
	/// # Counts to Symbols.
	fn llcl(&self) -> Result<[DeflateSym; N], ZopfliError>;

	/// # Symbols to Counts.
	fn llcl_symbols(lengths: &[DeflateSym; N]) -> Self;
}

/// # Helper: Generate LLCL method.
macro_rules! llcl {
	($maxbits:literal, $size:expr) => (
		/// # Counts to Symbols.
		fn llcl(&self) -> Result<[DeflateSym; $size], ZopfliError> {
			// Start the bitlengths at zero.
			let mut bitlengths = [DeflateSym::D00; $size];
			let bitcells = array_of_cells(&mut bitlengths);

			// Crunch!
			KATSCRATCH.with(|nodes| llcl::<$size, $maxbits>(self, bitcells, nodes))?;
			Ok(bitlengths)
		}
	);
}

/// # Helper: Generate LLCL symbols method.
macro_rules! llcl_symbols {
	($take:literal, $size:expr) => (
		#[inline]
		/// # Symbols to Counts.
		fn llcl_symbols(lengths: &[DeflateSym; $size]) -> Self {
			// The lengths should have previously been limited.
			debug_assert!(lengths.iter().all(|&l| (l as usize) < $take));

			// Count up the codes by code length. (Note: the compiler doesn't
			// understand the lengths have been limited to $take. Of all the
			// different ways to get it to elide bounds checks, overallocating
			// scratch to 19 performs best.
			let mut scratch = ZEROED_COUNTS_TREE;
			for l in lengths.iter().copied() { scratch[l as usize] += 1; }

			// Find the numerical value of the smallest code for each code
			// length (up to $take).
			let mut code = 0;
			scratch[0] = 0;
			for c in scratch.iter_mut().take($take) {
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

impl LengthLimitedCodeLengths<19> for [u32; 19] {
	llcl!(7, 19);
	llcl_symbols!(8, 19);
}

impl LengthLimitedCodeLengths<ZOPFLI_NUM_D> for ArrayD<u32> {
	/// # Counts to Symbols.
	fn llcl(&self) -> Result<ArrayD<DeflateSym>, ZopfliError> {
		// Start the bitlengths at zero.
		let mut bitlengths = [DeflateSym::D00; ZOPFLI_NUM_D];
		let bitcells = array_of_cells(&mut bitlengths);

		// Crunch!
		let count = KATSCRATCH.with(|nodes| llcl::<ZOPFLI_NUM_D, 15>(self, bitcells, nodes))?;

		// To work around a bug in zlib 1.2.1 — fixed in 2005, haha — we need
		// to have at least two non-zero distance codes. Pad the beginning as
		// needed to reach the quota.
		if count < 2 {
			// Everything is zero; patch the first two entries.
			if count == 0 {
				bitlengths[0] = DeflateSym::D01;
				bitlengths[1] = DeflateSym::D01;
			}
			// The first is zero so patch it.
			else if bitlengths[0].is_zero() { bitlengths[0] = DeflateSym::D01; }
			// By process of elimination, the second is zero so patch it.
			else { bitlengths[1] = DeflateSym::D01; }
		}

		Ok(bitlengths)
	}

	llcl_symbols!(16, ZOPFLI_NUM_D);
}

impl LengthLimitedCodeLengths<ZOPFLI_NUM_LL> for ArrayLL<u32> {
	llcl!(15, ZOPFLI_NUM_LL);
	llcl_symbols!(16, ZOPFLI_NUM_LL);
}



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



/// # Node Scratch.
///
/// This is a super-cheap arena-like structure for holding all the temporary
/// data required for length-limited-code-length calculations. (Damn nodes and
/// their damn self-referential tails!)
///
/// This requires doing some fairly un-Rust-like things, but that would be
/// equally true of any third-party arena as well, and since we know the
/// particulars in advance, we can do it leaner and meaner ourselves.
///
/// Pre-allocating storage for the worst-case entails some overhead, but like
/// the library's other caches, this is only ever instantiated as a
/// thread-local static, so will benefit from lots and lots of reuse. ;)
struct KatScratch {
	/// # Leaf Buffer.
	leaves: NonNull<u8>,

	/// # List Buffer.
	lists: NonNull<u8>,

	/// # Node Buffer.
	nodes: NonNull<u8>,

	/// # Written Nodes.
	///
	/// This holds the current number of nodes, allowing us to add entries to
	/// the right spot as we go along.
	nodes_len: Cell<usize>,
}

impl Drop for KatScratch {
	#[expect(unsafe_code, reason = "For alloc.")]
	/// # Drop.
	///
	/// We might as well free the memory associated with the backing arrays
	/// before we go.
	fn drop(&mut self) {
		// Safety: dealloc(LAYOUT) is equal and opposite to the alloc(LAYOUT)
		// calls used to create them.
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
	/// limiting pass could generate if every node were passed through here
	/// and every leaf were used. Neither is strictly true in practice but
	/// better to go a little over than come up short!
	const MAX: usize = (2 * ZOPFLI_NUM_LL - 2) * 15;

	/// # Leaves Array Layout.
	const LEAVES_LAYOUT: Layout = Layout::new::<[Leaf<'_>; ZOPFLI_NUM_LL]>();

	/// # List Array Layout.
	const LIST_LAYOUT: Layout = Layout::new::<[List; 15]>();

	/// # Node Array Layout.
	const NODE_LAYOUT: Layout = Layout::new::<[Node; Self::MAX]>();

	#[expect(unsafe_code, reason = "For alloc.")]
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
	/// from maybe-uninitialized data.
	fn new() -> Self {
		// Safety: alloc requires unsafe, but NonNull makes sure it actually
		// happened.
		let leaves: NonNull<u8> = NonNull::new(unsafe { alloc(Self::LEAVES_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::LEAVES_LAYOUT));

		// Safety: alloc requires unsafe, but NonNull makes sure it actually
		// happened.
		let lists: NonNull<u8> = NonNull::new(unsafe { alloc(Self::LIST_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::LIST_LAYOUT));

		// Safety: alloc requires unsafe, but NonNull makes sure it actually
		// happened.
		let nodes: NonNull<u8> = NonNull::new(unsafe { alloc(Self::NODE_LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::NODE_LAYOUT));

		Self {
			leaves,
			lists,
			nodes,
			nodes_len: Cell::new(0),
		}
	}

	#[expect(unsafe_code, reason = "For pointer fuckery.")]
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
			assert!(N <= ZOPFLI_NUM_LL, "BUG: frequencies must have a length of 32 or 288.");
		}

		let mut len = 0;
		let ptr = self.leaves.cast::<Leaf<'_>>().as_ptr();
		for (frequency, bitlength) in frequencies.iter().copied().zip(bitlengths) {
			if let Some(frequency) = NonZeroU32::new(frequency) {
				// Safety: the maximum N is ZOPFLI_NUM_LL, so this will
				// always be in range.
				unsafe {
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

	#[expect(unsafe_code, reason = "For pointer fuckery.")]
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
			count: NonZeroU32::MIN,
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

	#[expect(unsafe_code, reason = "For pointer fuckery.")]
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
		debug_assert!(len <= 15, "BUG: MAXBITS must be 7 or 15.");

		// Create a `List` with two starting `Node`s, then copy it `len` times
		// into our backing array.

		// Safety: we verified the length is in range, and since we're
		// writing before reading, the return value will have been
		// initialized.
		unsafe {
			let list = self.init_list(weight1, weight2);
			let ptr = self.lists.cast::<List>().as_ptr();
			for i in 0..len {
				ptr.add(i).write(list);
			}

			// Return the slice corresponding to the values we just wrote!
			std::slice::from_raw_parts_mut(ptr, len)
		}
	}

	#[expect(unsafe_code, reason = "For pointer fuckery.")]
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

			// Safety: we just verified the length is in range, and because
			// we're writing before reading, we know our return will have
			// been initialized.
			unsafe {
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
	/// # Frequency.
	frequency: NonZeroU32,

	/// # Bitlength.
	bitlength: &'a Cell<DeflateSym>,
}

impl Eq for Leaf<'_> {}

impl Ord for Leaf<'_> {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { self.frequency.cmp(&other.frequency) }
}

impl PartialEq for Leaf<'_> {
	#[inline]
	fn eq(&self, other: &Self) -> bool { self.frequency == other.frequency }
}

impl PartialOrd for Leaf<'_> {
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
	/// # Chain One.
	lookahead0: &'static Node,

	/// # Chain Two.
	lookahead1: &'static Node,
}

impl List {
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
	/// # Weight (Frequency).
	weight: NonZeroU32,

	/// # Count.
	count: NonZeroU32,

	/// # Tail (Previous Node).
	tail: Option<&'static Node>,
}

impl Node {
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
				weight: NonZeroU32::MIN, // We'll never look at this value.
				count: last_count.saturating_add(1),
				tail: list_z.lookahead1.tail,
			}
		}
		else {
			Self {
				weight: NonZeroU32::MIN, // We'll never look at this value.
				count: last_count,
				tail: Some(list_y.lookahead1),
			}
		}
	}
}



#[expect(unsafe_code, reason = "For array recast.")]
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

/// # Crunch the Code Lengths.
///
/// This method serves as the closure for the caller's call to
/// `KATSCRATCH.with_borrow_mut()`. It does all that needs doing to get
/// the desired length-limited data into the provided `bitlengths`. The number
/// of non-zero leaves is returned.
fn llcl<'a, const N: usize, const MAXBITS: usize>(
	frequencies: &'a [u32; N],
	bitlengths: &'a [Cell<DeflateSym>; N],
	nodes: &KatScratch
) -> Result<usize, ZopfliError> {
	const {
		assert!(
			(MAXBITS == 7 && N == 19) ||
			(MAXBITS == 15 && (N == ZOPFLI_NUM_D || N == ZOPFLI_NUM_LL)),
			"BUG: invalid MAXBITS / N combination.",
		);
	}

	let leaves = nodes.leaves(frequencies, bitlengths);
	let leaves_len = leaves.len();
	if leaves_len <= 2 {
		for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
		return Ok(leaves_len);
	}

	// Set up the lists.
	let lists = nodes.lists(
		usize::min(MAXBITS, leaves_len - 1),
		leaves[0].frequency,
		leaves[1].frequency,
	);

	// In the last list, (2 * len_leaves - 2) active chains need to be
	// created. We have two already from initialization; each boundary_pm run
	// will give us another.
	for _ in 0..2 * leaves_len - 5 { llcl_boundary_pm(leaves, lists, nodes)?; }

	// Add the last chain and write the results! (Note: this can't fail; we'll
	// always have at least two lists.)
	if let Some([list_y, list_z]) = lists.last_chunk::<2>() {
		let node = Node::last(list_y, list_z, leaves);
		llcl_write::<MAXBITS>(node, leaves)?;
	}

	Ok(leaves_len)
}

/// # Write Code Lengths!
///
/// This is the final stage of the LLCL chain, where the results are
/// finally recorded!
fn llcl_write<const MAXBITS: usize>(mut node: Node, leaves: &[Leaf<'_>]) -> Result<(), ZopfliError> {
	const {
		assert!(MAXBITS == 7 || MAXBITS == 15, "BUG: MAXBITS must be 7 or 15.");
	}

	// Make sure we counted correctly before doing anything else.
	let mut last_count = node.count;
	debug_assert!(
		leaves.len() >= last_count.get() as usize,
		"BUG: the count exceeds the leaf length?!",
	);

	// Write the changes!
	let mut writer = leaves.iter().take(last_count.get() as usize).rev();
	for value in DeflateSym::nonzero_iter().take(MAXBITS) {
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

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or the combination of the
/// two chains from the previous list.
///
/// This typically involves a lot of recursion, starting with the last list,
/// working its way down to the first. The compiler isn't thrilled about that,
/// but it likes a loop of loops even less, so it is what it is. ;)
fn llcl_boundary_pm(leaves: &[Leaf<'_>], lists: &mut [List], nodes: &KatScratch)
-> Result<(), ZopfliError> {
	// This method should never be called with an empty list.
	let [rest @ .., current] = lists else { return Err(zopfli_error!()); };
	let last_count = current.lookahead1.count;
	let previous = rest.last();

	// Short circuit: if we've reached the end of the lists or the last leaf
	// frequency is less than the weighted sum of the previous list, bump the
	// count and stop the recursion.
	if let Some(last_leaf) = leaves.get(last_count.get() as usize) {
		if previous.is_none_or(|p| last_leaf.frequency < p.weight_sum()) {
			// Shift the lookahead and add a new node.
			current.lookahead0 = current.lookahead1;
			current.lookahead1 = nodes.push(Node {
				weight: last_leaf.frequency,
				count: last_count.saturating_add(1),
				tail: current.lookahead0.tail,
			})?;
			return Ok(());
		}
	}

	// The chains are used up; let's create more work for ourselves by
	// recusing down the lists!
	if let Some(previous) = previous {
		// Shift the lookahead and add a new node.
		current.lookahead0 = current.lookahead1;
		current.lookahead1 = nodes.push(Node {
			weight: previous.weight_sum(),
			count: last_count,
			tail: Some(previous.lookahead1),
		})?;

		// Repeat from the previous list… twice!
		llcl_boundary_pm(leaves, rest, nodes)?;
		llcl_boundary_pm(leaves, rest, nodes)?;
	}

	Ok(())
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



#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	/// # No Drop Checks.
	///
	/// `KatScratch` manually allocates several data structures, and manually
	/// deallocates them on drop. It does not, however, perform any
	/// drop-in-place-type actions on the pointers because it doesn't need to.
	///
	/// At least, it _shouldn't_ need to. Let's verify that!
	fn t_nodrop() {
		use std::mem::needs_drop;

		assert!(! needs_drop::<[Leaf<'_>; ZOPFLI_NUM_LL]>());
		assert!(! needs_drop::<[List; 15]>());
		assert!(! needs_drop::<[Node; KatScratch::MAX]>());
	}

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
