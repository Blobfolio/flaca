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
	zopfli_error,
	ZopfliError,
};



thread_local!(
	/// # Shared Arena.
	///
	/// Each call to `length_limited_code_lengths` generates a hefty
	/// list of recursive node chains. This helps mitigate the costs of
	/// reallocation.
	static BUMP: RefCell<Bump> = RefCell::new(Bump::with_capacity(32_768))
);



/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for symbols
/// with the given counts, limited by `MAXBITS` (either 7 or 15 in practice).
pub(crate) fn length_limited_code_lengths<const MAXBITS: usize, const SIZE: usize>(
	frequencies: &[usize; SIZE],
	bitlengths: &mut [u32; SIZE],
) -> Result<(), ZopfliError> {
	// SIZE is only ever going to be 19, 32, or 288 at runtime, but the
	// compiler won't necessarily bother to confirm that in advance. Our tests
	// go as low as 6, so let's just run with that.
	if SIZE < 6 { return Err(zopfli_error!()); }

	// For performance reasons the bitlengths are passed by reference, but
	// they should always be zero-filled by this point.
	debug_assert!(bitlengths.iter().all(|b| *b == 0));

	// Convert bitlengths to a slice-of-cells so we can chop it up willynilly
	// without losing writeability.
	let bitlengths = Cell::from_mut(bitlengths.as_mut_slice()).as_slice_of_cells();

	// Build up a collection of "leaves" by joining each non-zero frequency
	// with its corresponding bitlength.
	let mut raw_leaves = [Leaf { frequency: NonZeroUsize::MIN, bitlength: &bitlengths[0] }; SIZE];
	let mut len_leaves = 0;
	for (v, leaf) in frequencies.iter()
		.copied()
		.zip(bitlengths)
		.filter_map(|(frequency, bitlength)| NonZeroUsize::new(frequency).map(
			|frequency| Leaf { frequency, bitlength }
		))
		.zip(raw_leaves.iter_mut())
	{
		*leaf = v;
		len_leaves += 1;
	}

	// Shortcut: nothing to do!
	if len_leaves == 0 || SIZE < len_leaves { return Ok(()); }

	// Reslice to the leaves we're actually using.
	let leaves = &mut raw_leaves[..len_leaves];

	// Sortcut: weighting only applies when there are more than two leaves.
	if leaves.len() <= 2 {
		for leaf in leaves { leaf.bitlength.set(1); }
		return Ok(());
	}

	// Sort the leaves by frequency.
	leaves.sort();

	// Crunch!
	BUMP.with_borrow_mut(|nodes| llcl::<MAXBITS>(leaves, nodes))
}



#[derive(Clone, Copy)]
/// # Leaf.
///
/// This is a simple tuple containing a non-zero frequency and its companion
/// bitlength.
struct Leaf<'a> {
	frequency: NonZeroUsize,
	bitlength: &'a Cell<u32>,
}

impl<'a> Eq for Leaf<'a> {}

impl<'a> Ord for Leaf<'a> {
	fn cmp(&self, other: &Self) -> Ordering { self.frequency.cmp(&other.frequency) }
}

impl<'a> PartialEq for Leaf<'a> {
	fn eq(&self, other: &Self) -> bool { self.frequency == other.frequency }
}

impl<'a> PartialOrd for Leaf<'a> {
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
	/// # Rotate.
	fn rotate(&mut self) { self.lookahead0 = self.lookahead1; }

	/// # Weight Sum.
	const fn weight_sum(&self) -> usize {
		self.lookahead0.weight + self.lookahead1.weight
	}
}



#[derive(Clone)]
/// # Node.
struct Node<'a> {
	weight: usize,
	count: usize,
	tail: Cell<Option<&'a Node<'a>>>,
}



/// # Crunch the Code Lengths.
///
/// This method serves as the closure for the exported method's
/// `BUMP.with_borrow_mut()` call, abstracted here mainly just to improve
/// readability.
fn llcl<'a, const MAXBITS: usize>(
	leaves: &'a [Leaf<'a>],
	nodes: &'a mut Bump,
) -> Result<(), ZopfliError> {
	// This can't happen; it is just a reminder for the compiler.
	if leaves.len() < 3 || (1 << MAXBITS) < leaves.len() {
		return Err(zopfli_error!());
	}

	// Shrink maxbits if we have fewer (leaves - 1).
	let maxbits =
		if leaves.len() - 1 < MAXBITS { leaves.len() - 1 }
		else { MAXBITS };

	let lookahead0 = nodes.alloc(Node {
		weight: leaves[0].frequency.get(),
		count: 1,
		tail: Cell::new(None),
	});

	let lookahead1 = nodes.alloc(Node {
		weight: leaves[1].frequency.get(),
		count: 2,
		tail: Cell::new(None),
	});

	// The max MAXBITS is only 15, so we might as well just (slightly)
	// over-allocate an array to hold our lists.
	let mut raw_lists = [List { lookahead0, lookahead1 }; MAXBITS];
	let lists = &mut raw_lists[..maxbits];

	// In the last list, (2 * len_leaves - 2) active chains need to be
	// created. We have two already from initialization; each boundary_pm run
	// will give us another.
	for _ in 0..2 * leaves.len() - 5 {
		llcl_boundary_pm(leaves, lists, nodes)?;
	}

	llcl_finish(leaves, lists, nodes)?;

	// Please be kind, rewind!
	nodes.reset();

	Ok(())
}

/// # Boundary Package-Merge Step.
///
/// Add a new chain to the list, using either a leaf or combination of two
/// chains from the previous list.
fn llcl_boundary_pm<'a>(leaves: &'a [Leaf<'a>], lists: &mut [List<'a>], nodes: &'a Bump)
-> Result<(), ZopfliError> {
	// This method should never be called with an empty list.
	let [rest @ .., current] = lists else { return Err(zopfli_error!()); };
	let last_count = current.lookahead1.count;

	// We're at the beginning, which is the end since we're iterating in
	// reverse.
	if rest.is_empty() {
		if last_count < leaves.len() {
			// Shift the lookahead and add a new node.
			current.rotate();
			current.lookahead1 = nodes.alloc(Node {
				weight: leaves[last_count].frequency.get(),
				count: last_count + 1,
				tail: current.lookahead0.tail.clone(),
			});
		}
		return Ok(());
	}

	// Shift the lookahead.
	current.rotate();

	let previous = rest[rest.len() - 1];
	let weight_sum = previous.weight_sum();

	// Add a leaf and increment the count.
	if last_count < leaves.len() && leaves[last_count].frequency.get() < weight_sum {
		current.lookahead1 = nodes.alloc(Node {
			weight: leaves[last_count].frequency.get(),
			count: last_count + 1,
			tail: current.lookahead0.tail.clone(),
		});
		return Ok(());
	}

	// Update the tail.
	current.lookahead1 = nodes.alloc(Node {
		weight: weight_sum,
		count: last_count,
		tail: Cell::new(Some(previous.lookahead1)),
	});

	// Replace the used-up lookahead chains by recursing twice.
	llcl_boundary_pm(leaves, rest, nodes)?;
	llcl_boundary_pm(leaves, rest, nodes)
}

/// # Finish and Write Code Lengths!
///
/// Add the final chain to the list, then write the weighted counts to the
/// bitlengths.
fn llcl_finish<'a>(
	leaves: &'a [Leaf<'a>],
	lists: &'a mut [List<'a>],
	nodes: &'a Bump,
) -> Result<(), ZopfliError> {
	// This won't fail; we'll always have at least two lists.
	let [_rest @ .., list_y, list_z] = lists else { return Err(zopfli_error!()); };

	// Add one more chain or update the tail.
	let last_count = list_z.lookahead1.count;
	let weight_sum = list_y.weight_sum();
	if last_count < leaves.len() && leaves[last_count].frequency.get() < weight_sum {
		list_z.lookahead1 = nodes.alloc(Node {
			weight: 0,
			count: last_count + 1,
			tail: list_z.lookahead1.tail.clone(),
		});
	}
	else {
		list_z.lookahead1.tail.set(Some(list_y.lookahead1));
	}

	// Write the changes!
	let mut node = list_z.lookahead1;
	let mut last_count = node.count;

	// But make sure we counted correctly first!
	if leaves.len() < last_count { return Err(zopfli_error!()); }

	// Okay, now we can write them!
	let mut writer = leaves.iter().take(last_count).rev();
	let mut value = 1;
	while let Some(tail) = node.tail.get() {
		// Wait for a change in counts to write the values.
		if tail.count < last_count {
			for leaf in writer.by_ref().take(last_count - tail.count) {
				leaf.bitlength.set(value);
			}
			last_count = tail.count;
		}
		value += 1;
		node = tail;
	}

	// Write the final value to any remaining leaves.
	for leaf in writer { leaf.bitlength.set(value); }
	Ok(())
}



#[cfg(test)]
mod tests {
	use super::*;

	// The original zopfli has no unit tests, but the zopfli-rs Rust port has
	// a few. The 3/4/7/15 maxbit tests below have been adapted from those.
	// They work, so that's promising!
	// <https://github.com/zopfli-rs/zopfli/blob/main/src/katajainen.rs>

	#[test]
	fn t_kat3() {
		let f = [1, 1, 5, 7, 10, 14];
		let mut b = [0; 6];
		assert!(length_limited_code_lengths::<3, 6>(&f, &mut b).is_ok());
		assert_eq!(b, [3, 3, 3, 3, 2, 2]);
	}

	#[test]
	fn t_kat4() {
		let f = [1, 1, 5, 7, 10, 14];
		let mut b = [0; 6];
		assert!(length_limited_code_lengths::<4, 6>(&f, &mut b).is_ok());
		assert_eq!(b, [4, 4, 3, 2, 2, 2]);
	}

	#[test]
	fn t_kat7() {
		let f = [252, 0, 1, 6, 9, 10, 6, 3, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
		let mut b = [0; 19];
		assert!(length_limited_code_lengths::<7, 19>(&f, &mut b).is_ok());
		assert_eq!(b, [1, 0, 6, 4, 3, 3, 3, 5, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	}

	#[test]
	fn t_kat15() {
		let f = [
			0, 0, 0, 0, 0, 0, 18, 0, 6, 0, 12, 2, 14, 9, 27, 15,
			23, 15, 17, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		];
		let mut b = [0; 32];
		assert!(length_limited_code_lengths::<15, 32>(&f, &mut b).is_ok());
		assert_eq!(
			b,
			[
				0, 0, 0, 0, 0, 0, 3, 0, 5, 0, 4, 6, 4, 4, 3, 4,
				3, 3, 3, 4, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			]
		);
	}

	#[test]
	fn t_kat_limited() {
		// No frequencies.
		let mut f = [0; 19];
		let mut b = [0; 19];
		assert!(length_limited_code_lengths::<7, 19>(&f, &mut b).is_ok());
		assert_eq!(b, [0; 19]);

		// One frequency.
		f[2] = 10;
		b.fill(0);
		assert!(length_limited_code_lengths::<7, 19>(&f, &mut b).is_ok());
		assert_eq!(b, [0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);

		// Two frequencies.
		f[0] = 248;
		b.fill(0);
		assert!(length_limited_code_lengths::<7, 19>(&f, &mut b).is_ok());
		assert_eq!(b, [1, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
	}
}
