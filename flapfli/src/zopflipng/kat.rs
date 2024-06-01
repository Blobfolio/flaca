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



/// # Tree Lengths and Distances.
///
/// This struct is used for calculating the optimal DEFLATE tree size and/or
/// writing the tree data to the output.
///
/// Both involve doing (virtually) the same thing several times in a row, so
/// the centralized storage helps reduce a little bit of that overhead.
pub(crate) struct TreeLd<'a> {
	ll_lengths: &'a [DeflateSym; ZOPFLI_NUM_LL],
	d_lengths: &'a [DeflateSym; ZOPFLI_NUM_D],
	hlit: usize,
	hdist: usize,
}

impl<'a> TreeLd<'a> {
	/// # New.
	pub(crate) const fn new(
		ll_lengths: &'a [DeflateSym; ZOPFLI_NUM_LL],
		d_lengths: &'a [DeflateSym; ZOPFLI_NUM_D],
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
	pub(crate) fn calculate_tree_size(&self) -> Result<(u8, u32), ZopfliError> {
		let mut best_size = u32::MAX;
		let mut best_idx = 0;

		// Try every combination.
		for idx in 0..8 {
			let size = self.crunch(idx, None)?;
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
	pub(crate) fn encode_tree(&self, out: &mut ZopfliOut) -> Result<(), ZopfliError> {
		let (extra, _) = self.calculate_tree_size()?;
		self.crunch(extra, Some(out))?;
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
		let cl_lengths = length_limited_code_lengths_tree(&cl_counts)?;

		// Find the last non-zero count.
		let mut hclen = 15;
		while hclen > 0 && cl_counts[DeflateSym::TREE[hclen + 3] as usize] == 0 {
			hclen -= 1;
		}

		// Write the results?
		if let Some(out) = out {
			// Convert the lengths to symbols.
			let cl_symbols = make_symbols(&cl_lengths)?;

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
		// Just calculate the would-be size and return.
		else {
			let mut size = 14;              // hlit, hdist, hclen.
			size += (hclen as u32 + 4) * 3;        // cl_lengths.
			for (a, b) in cl_lengths.iter().copied().zip(cl_counts.iter().copied()) {
				size += (a as u32) * b;
			}
			size += cl_counts[16] * 2; // Extra bits.
			size += cl_counts[17] * 3;
			size += cl_counts[18] * 7;
			Ok(size)
		}
	}
}



/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for tree
/// symbols with the given counts.
fn length_limited_code_lengths_tree(frequencies: &[u32; 19])
-> Result<[DeflateSym; 19], ZopfliError> {
	// Convert bitlengths to a slice-of-cells so we can chop it up willynilly
	// without losing writeability.
	let mut bitlengths = [DeflateSym::D00; 19];
	let bitcells = Cell::from_mut(bitlengths.as_mut_slice()).as_slice_of_cells();

	// Build up a collection of "leaves" by joining each non-zero frequency
	// with its corresponding bitlength.
	let leaves = make_leaves(frequencies, bitcells);

	// Sortcut: weighting only applies when there are more than two leaves.
	if leaves.len() <= 2 {
		for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
		return Ok(bitlengths);
	}

	// Crunch!
	SCRATCH.with(|nodes| llcl::<7>(&leaves, nodes)).map(|()| bitlengths)
}

/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for length
/// and distance symbols.
pub(crate) fn length_limited_code_lengths<const SIZE: usize>(
	frequencies: &[u32; SIZE],
	bitlengths: &mut [DeflateSym; SIZE],
) -> Result<(), ZopfliError> {
	// For performance reasons the bitlengths are passed by reference, but
	// they should always be zero-filled by this point.
	debug_assert!(bitlengths.iter().all(|b| b.is_zero()));

	// Convert bitlengths to a slice-of-cells so we can chop it up willynilly
	// without losing writeability.
	let bitlengths = Cell::from_mut(bitlengths.as_mut_slice()).as_slice_of_cells();

	// Build up a collection of "leaves" by joining each non-zero frequency
	// with its corresponding bitlength.
	let leaves = make_leaves(frequencies, bitlengths);

	// Sortcut: weighting only applies when there are more than two leaves.
	if leaves.len() <= 2 {
		for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
		return Ok(());
	}

	// Crunch!
	SCRATCH.with(|nodes| llcl::<15>(&leaves, nodes))
}



/// # Node Scratch.
///
/// This is a super-cheap arena-like structure for holding the thousands of
/// `Node` objects required for length-limited-code-length calculations.
///
/// The actual number of nodes will vary wildly from run-to-run, but this has
/// room enough for all eventualities.
struct KatScratch {
	data: NonNull<u8>,
	len: Cell<usize>,
}

impl Drop for KatScratch {
	#[allow(unsafe_code)]
	/// # Drop.
	///
	/// We might as well free the memory associated with the backing array
	/// before we go.
	fn drop(&mut self) {
		// Safety: this is the inverse of the allocation we performed when
		// constructing the object via `Self::new`.
		unsafe {
			std::alloc::dealloc(self.data.as_ptr().cast(), Self::LAYOUT);
		}
	}
}

impl KatScratch {
	/// # Max Elements.
	///
	/// This represents the theoretical maximum number of nodes a length-
	/// limiting pass might generate.
	const MAX: usize = (2 * ZOPFLI_NUM_LL - 2) * 15;

	/// # Layout.
	///
	/// This is what `self.data` actually looks like.
	const LAYOUT: Layout = Layout::new::<[Node; Self::MAX]>();

	#[allow(unsafe_code)]
	/// # New!
	///
	/// Return a new instance of self, allocated but uninitialized.
	///
	/// Similar to other mega-array structures like `ZopfliHash`, this is
	/// manually instantiated from pointers to avoid massive stack allocation.
	/// Unlike the others, however, the data will remain in pointer form for
	/// the duration to prevent the borrow-checker confusion this sort of
	/// structure would otherwise produce. (See `KatScratch::push` for more
	/// details.)
	fn new() -> Self {
		let data: NonNull<u8> = NonNull::new(unsafe { alloc(Self::LAYOUT) })
			.unwrap_or_else(|| handle_alloc_error(Self::LAYOUT));

		Self {
			data,
			len: Cell::new(0),
		}
	}

	#[allow(unsafe_code)]
	/// # Push.
	///
	/// Push a new node to the store and return an immutable reference to it.
	///
	/// This is a rather un-Rust thing to do, but works because:
	/// 1. Referenced values are never mutated;
	/// 2. The backing storage is never reallocated;
	/// 3. Values from a previous length-limited call are never re-accessed after that call has ended;
	fn push(&self, node: Node) -> Result<&'static Node, ZopfliError> {
		// Pull the current length and verify we have room to add more nodes.
		// This should never not be true, but zopfli is confusing so caution is
		// appropriate. Haha.
		let len = self.len.get();
		if len < Self::MAX {
			// Increment the length for next time.
			self.len.set(len + 1);

			unsafe {
				// Safety: we just verified the length is in range, and because
				// we're writing before reading, we know our return will have
				// been initialized.
				let ptr = self.data.cast::<Node>().as_ptr().add(len);
				ptr.write(node); // Copy the value into position.
				Ok(&*ptr)        // Return a reference to it.
			}
		}
		// If we somehow surpassed the theoretical maximum, return an error to
		// abort further processing of this image.
		else { Err(zopfli_error!()) }
	}

	/// # Reset.
	///
	/// `Node` is `Copy` so we can simply write new entries over top the old
	/// ones. Accordingly, this resets the internal length counter to zero.
	fn reset(&self) { self.len.set(0) }
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



#[allow(clippy::similar_names)]
/// # Crunch the Code Lengths.
///
/// This method serves as the closure for the exported method's
/// `BUMP.with_borrow_mut()` call, abstracted here mainly just to improve
/// readability.
fn llcl<const MAXBITS: usize>(leaves: &[Leaf<'_>], nodes: &KatScratch)
-> Result<(), ZopfliError> {
	// This can't happen; it is just a reminder for the compiler.
	if leaves.len() < 3 || (1 << MAXBITS) < leaves.len() {
		return Err(zopfli_error!());
	}

	// Reset before doing anything else so we have room for the nodes to come!
	nodes.reset();

	// Two starting nodes.
	let lookahead0 = nodes.push(Node {
		weight: leaves[0].frequency,
		count: NZ1,
		tail: None,
	})?;

	let lookahead1 = nodes.push(Node {
		weight: leaves[1].frequency,
		count: NZ2,
		tail: None,
	})?;

	// The max MAXBITS is only 15, so it's no big deal if we over-allocate
	// slightly.
	let mut raw_lists = [List { lookahead0, lookahead1 }; MAXBITS];
	let lists = &mut raw_lists[..MAXBITS.min(leaves.len() - 1)];

	// In the last list, (2 * len_leaves - 2) active chains need to be
	// created. We have two already from initialization; each boundary_pm run
	// will give us another.
	for _ in 0..2 * leaves.len() - 5 {
		llcl_boundary_pm(leaves, lists, nodes)?;
	}

	// Add the last chain and write the results!
	let node = llcl_finish(&lists[lists.len() - 2], &lists[lists.len() - 1], leaves);
	llcl_write(node, leaves)
}

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or combination of two
/// chains from the previous list.
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

/// # Finish Last Node!
///
/// This method establishes the final tail that the subsequent writing will
/// start with.
fn llcl_finish(list_y: &List, list_z: &List, leaves: &[Leaf<'_>]) -> Node {
	// Figure out the final node!
	let last_count = list_z.lookahead1.count;
	let weight_sum = list_y.weight_sum();
	if (last_count.get() as usize) < leaves.len() && leaves[last_count.get() as usize].frequency < weight_sum {
		Node {
			weight: NZ1, // We'll never look at this value.
			count: last_count.saturating_add(1),
			tail: list_z.lookahead1.tail,
		}
	}
	else {
		Node {
			weight: NZ1, // We'll never look at this value.
			count: last_count,
			tail: Some(list_y.lookahead1),
		}
	}
}

/// # Write Code Lengths!
fn llcl_write(mut node: Node, leaves: &[Leaf<'_>]) -> Result<(), ZopfliError> {
	// Make sure we counted correctly before doing anything else.
	let mut last_count = node.count;
	debug_assert!(leaves.len() >= last_count.get() as usize);

	// Write the changes!
	let mut writer = leaves.iter().take(last_count.get() as usize).rev();
	for value in DeflateSym::LIMITED {
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

/// # Make Leaves.
fn make_leaves<'a, const SIZE: usize>(
	frequencies: &'a [u32; SIZE],
	bitlengths: &'a [Cell<DeflateSym>],
) -> Vec<Leaf<'a>> {
	let mut out: Vec<_> = frequencies.iter()
		.copied()
		.zip(bitlengths)
		.filter_map(|(frequency, bitlength)|
			NonZeroU32::new(frequency).map(|frequency| Leaf { frequency, bitlength })
		)
		.collect();
	out.sort();
	out
}

#[allow(unsafe_code)]
/// # Zopfli Lengths to Symbols.
///
/// This returns a new symbol array given the lengths, which are themselves
/// symbols, but of a different kind. Haha.
fn make_symbols(lengths: &[DeflateSym; 19])
-> Result<[u32; 19], ZopfliError> {
	// Count up the codes by code length.
	let mut counts: [u32; 8] = [0; 8];
	for l in lengths.iter().copied() {
		if (l as u8) < 8 { counts[l as usize] += 1; }
		else { return Err(zopfli_error!()); }
	}

	// Find the numerical value of the smallest code for each code length.
	counts[0] = 0;
	let mut code = 0;
	let mut next_code: [u32; 8] = [0; 8];
	for i in 1..8 {
		code = (code + counts[i - 1]) << 1;
		next_code[i] = code;
	}

	// Update the symbols accordingly.
	let mut symbols = [0; 19];
	for (s, l) in symbols.iter_mut().zip(lengths.iter().copied()) {
		if ! l.is_zero() {
			// Safety: we already checked all lengths are less than MAXBITS.
			*s = unsafe { *next_code.get_unchecked(l as usize) };
			next_code[l as usize] += 1;
		}
	}
	Ok(symbols)
}



#[cfg(test)]
mod tests {
	use super::*;

	// These tests have been adapted from the zopfli-rs crate:
	// <https://github.com/zopfli-rs/zopfli/blob/main/src/katajainen.rs>

	#[test]
	fn t_kat7() {
		let f = [252, 0, 1, 6, 9, 10, 6, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
		assert_eq!(
			length_limited_code_lengths_tree(&f),
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
		let mut b = [DeflateSym::D00; 32];
		assert!(length_limited_code_lengths(&f, &mut b).is_ok());
		assert_eq!(
			b,
			[
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D03, DeflateSym::D00,
				DeflateSym::D05, DeflateSym::D00, DeflateSym::D04, DeflateSym::D06,
				DeflateSym::D04, DeflateSym::D04, DeflateSym::D03, DeflateSym::D04,
				DeflateSym::D03, DeflateSym::D03, DeflateSym::D03, DeflateSym::D04,
				DeflateSym::D06, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
				DeflateSym::D00, DeflateSym::D00, DeflateSym::D00, DeflateSym::D00,
			]
		);
	}

	#[test]
	fn t_kat_limited() {
		// No frequencies.
		let mut f = [0; 19];
		assert_eq!(
			length_limited_code_lengths_tree(&f),
			Ok([DeflateSym::D00; 19]),
		);

		// One frequency.
		f[2] = 10;
		assert_eq!(
			length_limited_code_lengths_tree(&f),
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
			length_limited_code_lengths_tree(&f),
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
