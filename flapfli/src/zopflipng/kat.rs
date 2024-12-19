/*!
# Flapfli: Katajainen and Tree-Related Business.

This module contains the Katajainen minimum-redundancy length-limited
code-writing logic — just as messy as it sounds! — as well as helpers related
to DEFLATE tree construction.
*/

use std::{
	alloc::{
		alloc,
		Layout,
	},
	cell::Cell,
	cmp::Ordering,
	num::{
		NonZeroU16,
		NonZeroU32,
	},
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



/// # Two is Non-Zero.
const NZ02: NonZeroU16 = NonZeroU16::new(2).unwrap();

/// # Fourteen is Non-Zero.
const NZ14: NonZeroU32 = NonZeroU32::new(14).unwrap();

/// # Zero-Filled Tree Counts.
const ZEROED_COUNTS_TREE: [u32; 19] = [0; 19];



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

			// First build up the leaves.
			let mut leaves: Box<[Leaf]> = self.iter()
				.copied()
				.zip(bitcells)
				.filter_map(|(f, bitlength)|
					NonZeroU32::new(f).map(|frequency| Leaf { frequency, bitlength })
				)
				.collect();
			let leaves_len = leaves.len();
			if leaves_len <= 2 {
				for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
				return Ok(bitlengths);
			}

			// Get the lists going.
			leaves.sort();
			let mut lists: [NodePair<$maxbits>; $maxbits] = [
				NodePair::new(leaves[0].frequency, leaves[1].frequency);
				$maxbits
			];

			// If the last leaf index is less than MAXBITS, we can reduce the PM
			// efforts accordingly.
			let sized_lists: &mut [NodePair<$maxbits>] =
				if leaves_len - 1 < $maxbits { &mut lists[$maxbits - (leaves_len - 1)..] }
				else { lists.as_mut_slice() };

			// We ultimately want (2 * len_leaves - 2) active chains in the last list.
			// Initialization gave us two; each PM pass will give us another.
			for _ in 0..2 * leaves_len - 5 { llcl_boundary_pm(&leaves, sized_lists)?; }

			// Fetch the final count and tail, then write the results!
			let (count, tail) = llcl_boundary_finish(&leaves, &lists);
			llcl_write(&leaves, count, tail)?;

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

		// First build up the leaves.
		let mut leaves: Box<[Leaf]> = self.iter()
			.copied()
			.zip(bitcells)
			.filter_map(|(f, bitlength)|
				NonZeroU32::new(f).map(|frequency| Leaf { frequency, bitlength })
			)
			.collect();
		let leaves_len = leaves.len();
		if leaves_len <= 2 {
			// To work around a bug in zlib 1.2.1 — fixed in 2005, haha — we
			// need to have at least two non-zero distance codes. Pad the
			// beginning as needed to reach the quota.
			if leaves_len == 0 {
				bitlengths[0] = DeflateSym::D01;
				bitlengths[1] = DeflateSym::D01;
			}
			else {
				for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
				if leaves_len == 1 {
					// The first is zero; flip it to make two.
					if bitlengths[0].is_zero() { bitlengths[0] = DeflateSym::D01; }
					// By process of elimination, the second is zero, so flip
					// it instead.
					else { bitlengths[1] = DeflateSym::D01; }
				}
			}
			return Ok(bitlengths);
		}

		// Get the lists going.
		leaves.sort();
		let mut lists: [NodePair<15>; 15] = [
			NodePair::new(leaves[0].frequency, leaves[1].frequency);
			15
		];

		// If the last leaf index is less than MAXBITS, we can reduce the PM
		// efforts accordingly.
		let sized_lists: &mut [NodePair<15>] =
			if leaves_len - 1 < 15 { &mut lists[15 - (leaves_len - 1)..] }
			else { lists.as_mut_slice() };

		// We ultimately want (2 * len_leaves - 2) active chains in the last list.
		// Initialization gave us two; each PM pass will give us another.
		for _ in 0..2 * leaves_len - 5 { llcl_boundary_pm(&leaves, sized_lists)?; }

		// Fetch the final count and tail, then write the results!
		let (count, tail) = llcl_boundary_finish(&leaves, &lists);
		llcl_write(&leaves, count, tail)?;

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



#[derive(Clone, Copy)]
/// # LLCL Leaf.
///
/// This is a simple tuple containing a non-zero frequency and its companion
/// bitlength, used for length-limited-code-length crunching.
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
/// # LLCL Node.
///
/// This holds the information for a single length-limited-code-length "node".
struct Node<const MAXBITS: usize> {
	/// # Weight.
	weight: NonZeroU32,

	/// # Count.
	count: NonZeroU16,

	/// # Tail.
	tail: NodeTail<MAXBITS>,
}



#[derive(Clone, Copy)]
/// # LLCL Node Tail.
///
/// This holds the counts — the only info that matters — of all previous nodes
/// occupying a given place in the chain.
struct NodeTail<const MAXBITS: usize>([Option<NonZeroU16>; MAXBITS]);



#[derive(Clone, Copy)]
/// # LLCL Node Pair.
///
/// This holds a pair of node chains for length-limited-code-length crunching.
struct NodePair<const MAXBITS: usize> {
	/// # Chain One.
	chain0: Node<MAXBITS>,

	/// # Chain Two.
	chain1: Node<MAXBITS>,
}

impl<const MAXBITS: usize> NodePair<MAXBITS> {
	/// # Generic Starter.
	///
	/// Initialize a new pair using the first two leaf weights and sequential
	/// counts.
	const fn new(weight1: NonZeroU32, weight2: NonZeroU32) -> Self {
		Self {
			chain0: Node {
				weight: weight1,
				count: NonZeroU16::MIN,
				tail: NodeTail([None; MAXBITS]),
			},
			chain1: Node {
				weight: weight2,
				count: NZ02,
				tail: NodeTail([None; MAXBITS]),
			},
		}
	}

	/// # Weight Sum.
	///
	/// Return the combined weight of both chains.
	const fn weight_sum(&self) -> NonZeroU32 {
		self.chain0.weight.saturating_add(self.chain1.weight.get())
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

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or the combination of the
/// two chains from the previous list.
///
/// This typically involves a lot of recursion, starting with the last list,
/// working its way down to the first. The compiler isn't thrilled about that,
/// but it likes a loop of loops even less, so it is what it is. ;)
fn llcl_boundary_pm<const MAXBITS: usize>(
	leaves: &[Leaf<'_>],
	lists: &mut [NodePair<MAXBITS>],
) -> Result<(), ZopfliError> {
	const {
		assert!(MAXBITS == 7 || MAXBITS == 15, "BUG: invalid MAXBITS");
	}

	// This method should never be called with an empty list.
	let [rest @ .., current] = lists else { return Err(zopfli_error!()); };
	let last_count = current.chain1.count;
	let previous = rest.last();

	// Short circuit: if we've reached the end of the lists or the last leaf
	// frequency is less than the weighted sum of the previous list, bump the
	// count and stop the recursion.
	if let Some(last_leaf) = leaves.get(last_count.get() as usize) {
		if previous.is_none_or(|p| last_leaf.frequency < p.weight_sum()) {
			// Rotate the lookaheads and add a new node to the end.
			current.chain0 = current.chain1;
			current.chain1.weight = last_leaf.frequency;
			current.chain1.count = last_count.saturating_add(1);
			return Ok(());
		}
	}

	// The chains are used up; let's create more work for ourselves by
	// recusing down the lists!
	if let Some(previous) = previous {
		// Rotate the lookaheads and add a new node to the end.
		current.chain0 = current.chain1;
		current.chain1.weight = previous.weight_sum();
		current.chain1.count = last_count;
		current.chain1.tail = previous.chain1.tail;
		current.chain1.tail.0.copy_within(..MAXBITS - 1, 1);
		current.chain1.tail.0[0].replace(previous.chain1.count);

		// Repeat from the previous list… twice!
		llcl_boundary_pm(leaves, rest)?;
		llcl_boundary_pm(leaves, rest)?;
	}

	Ok(())
}

#[inline]
/// # Calculate Last Node.
///
/// This method calculates and returns the final node count and tail for
/// writing.
fn llcl_boundary_finish<const MAXBITS: usize>(
	leaves: &[Leaf<'_>],
	lists: &[NodePair<MAXBITS>; MAXBITS],
) -> (NonZeroU16, NodeTail<MAXBITS>) {
	const {
		assert!(MAXBITS == 7 || MAXBITS == 15, "BUG: invalid MAXBITS");
	}

	// Figure out the final node!
	let last_count = lists[MAXBITS - 1].chain1.count;
	let weight_sum = lists[MAXBITS - 2].weight_sum();
	if (last_count.get() as usize) < leaves.len() && leaves[last_count.get() as usize].frequency < weight_sum {
		(last_count.saturating_add(1), lists[MAXBITS - 1].chain1.tail)
	}
	else {
		let mut tail = lists[MAXBITS - 2].chain1.tail;
		tail.0.copy_within(..MAXBITS - 1, 1);
		tail.0[0].replace(lists[MAXBITS - 2].chain1.count);
		(last_count, tail)
	}
}

#[inline]
/// # Write Code Lengths!
///
/// This is the final stage of the LLCL chain, where the results are
/// actually recorded!
fn llcl_write<const MAXBITS: usize>(
	leaves: &[Leaf<'_>],
	mut last_count: NonZeroU16,
	counts: NodeTail<MAXBITS>,
) -> Result<(), ZopfliError> {
	const {
		assert!(MAXBITS == 7 || MAXBITS == 15, "BUG: invalid MAXBITS");
	}

	// Make sure we counted correctly before doing anything else.
	debug_assert!(
		leaves.len() >= last_count.get() as usize,
		"BUG: the count exceeds the leaf length?!",
	);

	// Write the changes!
	let mut writer = leaves.iter().take(last_count.get() as usize).rev();
	let mut reader = counts.0.into_iter().flatten();
	for value in DeflateSym::nonzero_iter().take(MAXBITS) {
		// Pull the next tail, if any.
		if let Some(tail) = reader.next() {
			// Wait for a change in counts to write the values.
			if tail < last_count {
				for leaf in writer.by_ref().take((last_count.get() - tail.get()) as usize) {
					leaf.bitlength.set(value);
				}
				last_count = tail;
			}
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
