/*!
# Flaca: Zopfli Katajainen.

This module contains the Katajainen minimum-redundancy length-limited
code-writing logic, which is just as messy as it sounds. Haha.
*/

use bumpalo::Bump;
use std::{
	cell::{
		Cell,
		RefCell,
	},
	cmp::Ordering,
	num::NonZeroUsize,
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
const NZ1: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(1) };

#[allow(unsafe_code)]
const NZ2: NonZeroUsize = unsafe { NonZeroUsize::new_unchecked(2) };



thread_local!(
	/// # Node Arena.
	///
	/// Each and every call to the (two) length-limited-code-length methods has
	/// the potential to generate _thousands_ of self-referential `Node` chains,
	/// only to throw them away at the end of the function call. Haha.
	///
	/// This shared storage helps take some of the sting out of the ridiculous
	/// allocation overhead, at least on a per-thread basis.
	///
	/// Note: the initial capacity is set to about 10% of the theoretical
	/// maximum; bumpalo will bump as needed.
	static BUMP: RefCell<Bump> = RefCell::new(Bump::with_capacity(
		(2 * ZOPFLI_NUM_D - 2) * 15 * std::mem::size_of::<Node<'_>>()
	))
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
	pub(crate) fn calculate_tree_size(&mut self) -> Result<(u8, usize), ZopfliError> {
		let mut best_size = usize::MAX;
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
	pub(crate) fn encode_tree(&mut self, out: &mut ZopfliOut) -> Result<(), ZopfliError> {
		let (extra, _) = self.calculate_tree_size()?;
		self.crunch(extra, Some(out))?;
		Ok(())
	}

	#[allow(clippy::cast_possible_truncation)]
	/// # Crunch the Tree.
	///
	/// This crunches the data for the given index, either returning the size
	/// or writing it to the output (and returning zero).
	fn crunch(&mut self, extra: u8, out: Option<&mut ZopfliOut>) -> Result<usize, ZopfliError> {
		// Are we using any of the special alphabet parts?
		let use_16 = 0 != extra & 1;
		let use_17 = 0 != extra & 2;
		let use_18 = 0 != extra & 4;

		// We need a structure to hold the counts for each symbol.
		let mut cl_counts = [0_usize; 19];

		// We also need a structure to hold the positional symbol/count data,
		// but only if we're going to write the tree at the end. If not, this
		// will remain unallocated (and hopefully have minimal runtime impact).
		let mut rle = Vec::new();
		if out.is_some() { rle.reserve(self.len()); }

		// Run through all the length symbols, then the distance symbols, with
		// the odd skip to keep us on our toes.
		let mut i = 0;
		while i < self.len() {
			let mut count = 1;
			let symbol = self.symbol(i);

			// Peek ahead; we may be able to do more in one go.
			if use_16 || (symbol.is_zero() && (use_17 || use_18)) {
				let mut j = i + 1;
				while j < self.len() && symbol == self.symbol(j) {
					count += 1;
					j += 1;
				}

				// Skip these indices, if any, on the next pass.
				i += count - 1;
			}

			// Repetitions of zeroes.
			if symbol.is_zero() && count >= 3 {
				if use_18 {
					while count >= 11 {
						let count2 = count.min(138);
						if out.is_some() {
							rle.push((DeflateSym::D18, count2 as u32 - 11));
						}
						cl_counts[DeflateSym::D18 as usize] += 1;
						count -= count2;
					}
				}
				if use_17 {
					while count >= 3 {
						let count2 = count.min(10);
						if out.is_some() {
							rle.push((DeflateSym::D17, count2 as u32 - 3));
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
						rle.push((DeflateSym::D16, count2 as u32 - 3));
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
			size += (hclen + 4) * 3;        // cl_lengths.
			for (&a, b) in cl_lengths.iter().zip(cl_counts.iter()) {
				size += (a as usize) * b;
			}
			size += cl_counts[16] * 2; // Extra bits.
			size += cl_counts[17] * 3;
			size += cl_counts[18] * 7;
			Ok(size)
		}
	}
}



#[inline]
/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for tree
/// symbols with the given counts.
fn length_limited_code_lengths_tree(frequencies: &[usize; 19])
-> Result<[DeflateSym; 19], ZopfliError> {
	// Convert bitlengths to a slice-of-cells so we can chop it up willynilly
	// without losing writeability.
	let mut bitlengths = [DeflateSym::D00; 19];
	let bitcells = Cell::from_mut(bitlengths.as_mut_slice()).as_slice_of_cells();

	// Build up a collection of "leaves" by joining each non-zero frequency
	// with its corresponding bitlength.
	let mut raw_leaves = [Leaf { frequency: NonZeroUsize::MIN, bitlength: &bitcells[0] }; 19];
	let leaves = make_leaves(frequencies, bitcells, &mut raw_leaves);

	// Sortcut: weighting only applies when there are more than two leaves.
	if leaves.len() <= 2 {
		for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
		return Ok(bitlengths);
	}

	// Sort the leaves by frequency.
	leaves.sort();

	// Crunch!
	BUMP.with_borrow_mut(|nodes| llcl::<7>(leaves, nodes)).map(|()| bitlengths)
}

/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for length
/// and distance symbols.
pub(crate) fn length_limited_code_lengths<const SIZE: usize>(
	frequencies: &[usize; SIZE],
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
	let mut raw_leaves = [Leaf { frequency: NonZeroUsize::MIN, bitlength: &bitlengths[0] }; SIZE];
	let leaves = make_leaves(frequencies, bitlengths, &mut raw_leaves);

	// Sortcut: weighting only applies when there are more than two leaves.
	if leaves.len() <= 2 {
		for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }
		return Ok(());
	}

	// Sort the leaves by frequency.
	leaves.sort();

	// Crunch!
	BUMP.with_borrow_mut(|nodes| llcl::<15>(leaves, nodes))
}



#[derive(Clone, Copy)]
/// # Leaf.
///
/// This is a simple tuple containing a non-zero frequency and its companion
/// bitlength.
struct Leaf<'a> {
	frequency: NonZeroUsize,
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
struct List<'a> {
	lookahead0: &'a Node<'a>,
	lookahead1: &'a Node<'a>,
}

impl<'a> List<'a> {
	#[inline]
	/// # Rotate.
	fn rotate(&mut self) { self.lookahead0 = self.lookahead1; }

	#[inline]
	/// # Weight Sum.
	const fn weight_sum(&self) -> NonZeroUsize {
		self.lookahead0.weight.saturating_add(self.lookahead1.weight.get())
	}
}



#[derive(Clone)]
/// # Node.
struct Node<'a> {
	weight: NonZeroUsize,
	count: NonZeroUsize,
	tail: Cell<Option<&'a Node<'a>>>,
}



#[allow(clippy::similar_names)]
#[inline]
/// # Crunch the Code Lengths.
///
/// This method serves as the closure for the exported method's
/// `BUMP.with_borrow_mut()` call, abstracted here mainly just to improve
/// readability.
fn llcl<'a, const MAXBITS: usize>(
	leaves: &[Leaf<'a>],
	nodes: &'a mut Bump,
) -> Result<(), ZopfliError> {
	// This can't happen; it is just a reminder for the compiler.
	if leaves.len() < 3 || (1 << MAXBITS) < leaves.len() {
		return Err(zopfli_error!());
	}

	// Two starting nodes.
	let node0 = Node {
		weight: leaves[0].frequency,
		count: NZ1,
		tail: Cell::new(None),
	};

	let node1 = Node {
		weight: leaves[1].frequency,
		count: NZ2,
		tail: Cell::new(None),
	};

	// The max MAXBITS is only 15, so we might as well just (slightly)
	// over-allocate an array to hold our lists.
	let mut raw_lists = [List { lookahead0: &node0, lookahead1: &node1 }; MAXBITS];
	let lists = &mut raw_lists[..MAXBITS.min(leaves.len() - 1)];

	// In the last list, (2 * len_leaves - 2) active chains need to be
	// created. We have two already from initialization; each boundary_pm run
	// will give us another.
	for _ in 0..2 * leaves.len() - 5 {
		llcl_boundary_pm(leaves, lists, nodes)?;
	}

	// Add the last chain and write the results!
	llcl_finish(leaves, lists, nodes)?;

	// Please be kind, rewind!
	nodes.reset();

	Ok(())
}

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or combination of two
/// chains from the previous list.
fn llcl_boundary_pm<'a>(leaves: &[Leaf<'a>], lists: &mut [List<'a>], nodes: &'a Bump)
-> Result<(), ZopfliError> {
	// This method should never be called with an empty list.
	let [rest @ .., current] = lists else { return Err(zopfli_error!()); };
	let last_count = current.lookahead1.count;

	// We're at the beginning, which is the end since we're iterating in
	// reverse.
	if rest.is_empty() {
		if let Some(last_leaf) = leaves.get(last_count.get()) {
			// Shift the lookahead and add a new node.
			current.rotate();
			current.lookahead1 = nodes.try_alloc(Node {
				weight: last_leaf.frequency,
				count: last_count.saturating_add(1),
				tail: current.lookahead0.tail.clone(),
			}).map_err(|_| zopfli_error!())?;
		}
		return Ok(());
	}

	// Shift the lookahead.
	current.rotate();

	let previous = rest[rest.len() - 1];
	let weight_sum = previous.weight_sum();

	// Add a leaf and increment the count.
	if let Some(last_leaf) = leaves.get(last_count.get()) {
		if last_leaf.frequency < weight_sum {
			current.lookahead1 = nodes.try_alloc(Node {
				weight: last_leaf.frequency,
				count: last_count.saturating_add(1),
				tail: current.lookahead0.tail.clone(),
			}).map_err(|_| zopfli_error!())?;
			return Ok(());
		}
	}

	// Update the tail.
	current.lookahead1 = nodes.try_alloc(Node {
		weight: weight_sum,
		count: last_count,
		tail: Cell::new(Some(previous.lookahead1)),
	}).map_err(|_| zopfli_error!())?;

	// Replace the used-up lookahead chains by recursing twice.
	llcl_boundary_pm(leaves, rest, nodes)?;
	llcl_boundary_pm(leaves, rest, nodes)
}

#[inline]
/// # Finish and Write Code Lengths!
///
/// Add the final chain to the list, then write the weighted counts to the
/// bitlengths.
fn llcl_finish<'a>(
	leaves: &[Leaf<'a>],
	lists: &mut [List<'a>],
	nodes: &'a Bump,
) -> Result<(), ZopfliError> {
	// This won't fail; we'll always have at least two lists.
	let [_rest @ .., list_y, list_z] = lists else { return Err(zopfli_error!()); };

	// Add one more chain or update the tail.
	let last_count = list_z.lookahead1.count;
	let weight_sum = list_y.weight_sum();
	if last_count.get() < leaves.len() && leaves[last_count.get()].frequency < weight_sum {
		list_z.lookahead1 = nodes.try_alloc(Node {
			weight: NZ1, // We'll never look at this value.
			count: last_count.saturating_add(1),
			tail: list_z.lookahead1.tail.clone(),
		}).map_err(|_| zopfli_error!())?;
	}
	else {
		list_z.lookahead1.tail.set(Some(list_y.lookahead1));
	}

	// Write the changes!
	let mut node = list_z.lookahead1;
	let mut last_count = node.count;

	// But make sure we counted correctly first!
	debug_assert!(leaves.len() >= last_count.get());

	// Okay, now we can write them!
	let mut writer = leaves.iter().take(last_count.get()).rev();
	for value in DeflateSym::LIMITED {
		// Pull the next tail, if any.
		if let Some(tail) = node.tail.get() {
			// Wait for a change in counts to write the values.
			if tail.count < last_count {
				for leaf in writer.by_ref().take(last_count.get() - tail.count.get()) {
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

#[inline]
/// # Make Leaves.
fn make_leaves<'a, const SIZE: usize>(
	frequencies: &'a [usize; SIZE],
	bitlengths: &'a [Cell<DeflateSym>],
	leaves: &'a mut [Leaf<'a>],
) -> &'a mut [Leaf<'a>] {
	let mut len_leaves = 0;
	for (v, leaf) in frequencies.iter()
		.copied()
		.zip(bitlengths)
		.filter_map(|(frequency, bitlength)| NonZeroUsize::new(frequency).map(
			|frequency| Leaf { frequency, bitlength }
		))
		.zip(leaves.iter_mut())
	{
		*leaf = v;
		len_leaves += 1;
	}

	// Reslice to the leaves we're actually using.
	&mut leaves[..len_leaves]
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
