/*!
# Flapfli: Length Limited Code Lengths.

The `LengthLimitedCodeLengths` requires a lot of supporting infrastructure…
*/

use std::{
	cell::Cell,
	cmp::Ordering,
	num::{
		NonZeroU8,
		NonZeroU16,
		NonZeroU32,
	},
};
use super::{
	DeflateSym,
	ZEROED_COUNTS_TREE,
	zopfli_error,
	ZOPFLI_NUM_D,
	ZOPFLI_NUM_LL,
	ZopfliError,
};



/// # Length Limited Code Lengths.
///
/// This trait adds an `llcl` method to symbol count arrays that generates the
/// appropriate deflate symbols (bitlengths), and a `llcl_symbols` method that
/// does something like the reverse.
pub(crate) trait LengthLimitedCodeLengths<const N: usize> {
	/// # Counts to Symbols.
	fn llcl(&self) -> Result<[DeflateSym; N], ZopfliError>;

	/// # Symbols to Counts.
	fn llcl_symbols(lengths: &[DeflateSym; N]) -> [u32; N];
}



/// # Length Limited Code Lengths (More).
///
/// This holds a few internal LLCL-related methods that don't need to be
/// exposed outside this module.
trait LengthLimitedCodeLengthsExt<Count: NodeCount, const MAXBITS: usize>: Sized {
	/// # Boundary Package-Merge Step.
	///
	/// Add a new chain to the list, using either a leaf or the combination
	/// of the two chains from the previous list.
	///
	/// This typically involves a lot of recursion, starting with the last
	/// list, working its way down to the first. The compiler isn't
	/// thrilled about that, but it likes a loop of loops even less, so it
	/// is what it is. ;)
	fn llcl_boundary_pm(
		leaves: &[Leaf],
		lists: &mut [NodePair<Count, MAXBITS>],
	) -> Result<(), ZopfliError>;

	/// # Write Code Lengths!
	///
	/// This is the final stage of the LLCL chain, where the results are
	/// actually recorded!
	fn llcl_write(
		leaves: &[Leaf],
		last_count: Count,
		counts: NodeTail<Count, MAXBITS>,
	) -> Result<(), ZopfliError>;
}



/// # Helper: Implement the LLCL Traits.
///
/// Rust generics/specialization support isn't quite where we need it to be
/// yet, so macros it is!
macro_rules! llcl {
	($size:expr, $maxbits:literal, $count:ty) => (
		impl LengthLimitedCodeLengths<$size> for [u32; $size] {
			/// # Counts to Symbols.
			fn llcl(&self) -> Result<[DeflateSym; $size], ZopfliError> {
				// Start the bitlengths at zero.
				let mut bitlengths = [DeflateSym::D00; $size];
				let bitcells = array_of_cells(&mut bitlengths);

				// First build up the leaves by joining non-zero frequencies with
				// their corresponding bitlengths. There will almost certainly be
				// fewer than N leaves, but copy is cheap; we'll fix the sizing in
				// post! Haha.
				let mut leaves_len = 0_usize;
				let mut raw_leaves = [
					Leaf { frequency: NonZeroU32::MIN, bitlength: &bitcells[0] };
					$size
				];
				for (frequency, bitlength) in self.iter().copied().zip(bitcells) {
					if let Some(frequency) = NonZeroU32::new(frequency) {
						raw_leaves[leaves_len] = Leaf { frequency, bitlength };
						leaves_len += 1;
					}
				}
				let leaves = &mut raw_leaves[..leaves_len]; // Actual leaves.

				// The boundary nonsense requires at least three leaves, so if
				// we came up short we can just assume a distance of "one" and
				// bail early.
				if leaves_len <= 2 {
					for leaf in leaves { leaf.bitlength.set(DeflateSym::D01); }

					// To work around a bug in zlib 1.2.1 — fixed in 2005, haha — we
					// need to have at least two non-zero distance codes. (Padding the
					// beginning does the trick.)
					//
					// The compiler should hopefully be smart enough to optimize this
					// out for the 19/ZOPFLI_NUM_LL implementations.
					if $size == ZOPFLI_NUM_D {
						// All leaves are zero; flip the first two.
						if leaves_len == 0 {
							bitlengths[0] = DeflateSym::D01;
							bitlengths[1] = DeflateSym::D01;
						}
						// One leaf is zero.
						else if leaves_len == 1 {
							// The first is zero, so flip it to get two.
							if bitlengths[0].is_zero() { bitlengths[0] = DeflateSym::D01; }
							// Then the second must be zero, so flip it
							// instead.
							else { bitlengths[1] = DeflateSym::D01; }
						}
					}

					return Ok(bitlengths);
				}

				// Get the lists going.
				leaves.sort();
				let mut raw_lists = [
					NodePair::<$count, $maxbits>::new(leaves[0].frequency, leaves[1].frequency);
					$maxbits
				];

				// MAXBITS gives us an upper limit, but if we have fewer leaves
				// than that we can shrink it to match.
				let lists = &mut raw_lists[$maxbits.saturating_sub(leaves_len - 1)..];

				// We ultimately want (2 * len_leaves - 2) active chains in the last list.
				// Initialization gave us two; each PM pass will give us another.
				for _ in 0..2 * leaves_len - 5 { Self::llcl_boundary_pm(leaves, lists)?; }

				// Fetch the final count and tail, then write the results!
				let (count, tail) = NodeTail::<$count, $maxbits>::last(leaves, &raw_lists);
				Self::llcl_write(leaves, count, tail)?;

				Ok(bitlengths)
			}

			#[inline]
			/// # Symbols to Counts.
			fn llcl_symbols(lengths: &[DeflateSym; $size]) -> [u32; $size] {
				// The lengths should have previously been limited.
				debug_assert!(
					lengths.iter().all(|&l| (l as usize) < $maxbits + 1),
					"BUG: LLCL symbol lengths out of range."
				);

				// Count up the codes by code length. (Note: the compiler doesn't
				// understand the lengths have been limited to MAXBITS+1. Of all the
				// different ways to get it to elide bounds checks, overallocating
				// scratch to 19 performs best.
				let mut scratch = ZEROED_COUNTS_TREE;
				for l in lengths.iter().copied() { scratch[l as usize] += 1; }

				// Find the numerical value of the smallest code for each code
				// length (up to MAXBITS+1).
				let mut code = 0;
				scratch[0] = 0;
				for c in scratch.iter_mut().take($maxbits + 1) {
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
		}

		impl LengthLimitedCodeLengthsExt<$count, $maxbits> for [u32; $size] {
			/// # Boundary Package-Merge Step.
			fn llcl_boundary_pm(
				leaves: &[Leaf],
				lists: &mut [NodePair<$count, $maxbits>],
			) -> Result<(), ZopfliError> {
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
					current.chain1.tail = previous.chain1.as_tail();

					// Repeat from the previous list… twice!
					Self::llcl_boundary_pm(leaves, rest)?;
					Self::llcl_boundary_pm(leaves, rest)?;
				}

				Ok(())
			}

			#[inline]
			/// # Write Code Lengths!
			fn llcl_write(
				leaves: &[Leaf],
				mut last_count: $count,
				counts: NodeTail<$count, $maxbits>,
			) -> Result<(), ZopfliError> {
				// Make sure we counted correctly before doing anything else.
				debug_assert!(
					leaves.len() >= last_count.get() as usize,
					"BUG: the count exceeds the leaf length?!",
				);

				// Write the changes!
				let mut writer = leaves.iter().take(last_count.get() as usize).rev();
				let mut reader = counts.0.into_iter().flatten();
				for value in DeflateSym::nonzero_iter().take($maxbits) {
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
		}

		impl NodeTail<$count, $maxbits> {
			/// # Calculate Last Node.
			///
			/// This method calculates and returns the final node count and
			/// tail for writing.
			const fn last(
				leaves: &[Leaf],
				lists: &[NodePair<$count, $maxbits>; $maxbits],
			) -> ($count, Self) {
				// Figure out the final node!
				let last_count = lists[$maxbits - 1].chain1.count;
				let weight_sum = lists[$maxbits - 2].weight_sum();
				if
					(last_count.get() as usize) < leaves.len() &&
					leaves[last_count.get() as usize].frequency.get() < weight_sum.get()
				{
					(last_count.saturating_add(1), lists[$maxbits - 1].chain1.tail)
				}
				else {
					(last_count, lists[$maxbits - 2].chain1.as_tail())
				}
			}
		}
	);
}
llcl!(19, 7_usize, NonZeroU8);
llcl!(ZOPFLI_NUM_D, 15_usize, NonZeroU8);
llcl!(ZOPFLI_NUM_LL, 15_usize, NonZeroU16);



/// # LLCL Counting Types.
///
/// This trait is a workaround to enable us to count nodes with one byte or
/// two depending on the size of the input array. (`ZOPFLI_NUM_LL` is slightly
/// too big for `NonZeroU8`. _Annoying!_)
///
/// It might sound silly to quibble over a byte, but nodes and tails are
/// created and destroyed repeatedly during crunching, so the savings add up
/// quickly.
///
/// TODO: refactor if/when `ZeroablePrimitive` becomes stable; we aren't
/// doing anything special here.
trait NodeCount: Copy + Eq + Ord + Sized {
	/// # One.
	const ONE: Self;

	/// # Two.
	const TWO: Self;
}

/// # Helper: Implement `NodeCount`.
macro_rules! node_count {
	($ty:ty) => (
		impl NodeCount for $ty {
			/// # One.
			const ONE: Self = <$ty>::MIN;

			/// # Two.
			const TWO: Self = <$ty>::new(2).unwrap();
		}
	);
}
node_count!(NonZeroU8);
node_count!(NonZeroU16);



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
struct Node<Count: NodeCount, const MAXBITS: usize> {
	/// # Weight.
	weight: NonZeroU32,

	/// # Count.
	count: Count,

	/// # Tail.
	tail: NodeTail<Count, MAXBITS>,
}

impl Node<NonZeroU8, 7> {
	#[expect(clippy::many_single_char_names, reason = "For readability.")]
	#[inline]
	/// # As Tail.
	///
	/// Return a copy of `self.tail` with `self.count` prepended to it.
	const fn as_tail(&self) -> NodeTail<NonZeroU8, 7> {
		let [b, c, d, e, f, g, _] = self.tail.0;
		NodeTail([Some(self.count), b, c, d, e, f, g])
	}
}

impl<Count: NodeCount> Node<Count, 15> {
	#[expect(clippy::many_single_char_names, reason = "For readability.")]
	#[inline]
	/// # As Tail.
	///
	/// Return a copy of `self.tail` with `self.count` prepended to it.
	const fn as_tail(&self) -> NodeTail<Count, 15> {
		let [b, c, d, e, f, g, h, i, j, k, l, m, n, o, _] = self.tail.0;
		NodeTail([Some(self.count), b, c, d, e, f, g, h, i, j, k, l, m, n, o])
	}
}



#[derive(Clone, Copy)]
/// # LLCL Node Tail.
///
/// This holds a list of immutable node count(s) occupying a given position in
/// the chain.
///
/// Most zopfli implementations use arena-like structures for nodes to enable
/// self-referential tails, but in practice holding onto all those temporary
/// nodes just to keep their references (if any) valid undermines the cheapness
/// of the pointers themselves.
///
/// (Nodes and tails are created and destroyed with impunity during crunching;
/// it's very wasteful. Haha.)
///
/// Since tails only actually need to record unchanging node _counts_, and tail
/// lengths are constrained by `MAXBITS` — 7 or 15 — redundant `Copy`-friendly
/// arrays offer a more efficient and performant alternative.
struct NodeTail<Count: NodeCount, const MAXBITS: usize>([Option<Count>; MAXBITS]);



#[derive(Clone, Copy)]
/// # LLCL Node Pair.
///
/// This holds a pair of node chains for length-limited-code-length crunching.
struct NodePair<Count: NodeCount, const MAXBITS: usize> {
	/// # Chain One.
	chain0: Node<Count, MAXBITS>,

	/// # Chain Two.
	chain1: Node<Count, MAXBITS>,
}

impl<Count: NodeCount, const MAXBITS: usize> NodePair<Count, MAXBITS> {
	/// # Generic Starter.
	///
	/// Initialize a new pair using the first two leaf weights and sequential
	/// counts.
	const fn new(weight1: NonZeroU32, weight2: NonZeroU32) -> Self {
		Self {
			chain0: Node {
				weight: weight1,
				count: Count::ONE,
				tail: NodeTail([None; MAXBITS]),
			},
			chain1: Node {
				weight: weight2,
				count: Count::TWO,
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

