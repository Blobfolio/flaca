/*!
# Flaca: Zopfli Katajainen
*/

use bumpalo::Bump;
use std::{
	cell::{
		Cell,
		RefCell,
	},
	cmp::Ordering,
	mem::MaybeUninit,
};



thread_local!(
	/// # Shared Arena.
	///
	/// Each call to `ZopfliLengthLimitedCodeLengths` generates a hefty list
	/// of recursive node chains. This helps mitigate the costs of
	/// reallocation.
	static BUMP: RefCell<Bump> = RefCell::new(Bump::with_capacity(32_768))
);



#[no_mangle]
#[allow(unsafe_code)]
/// # Length Limited Code Lengths (FFI).
///
/// This is only called in one place from C with fixed sizes.
pub(crate) extern "C" fn ZopfliLengthLimitedCodeLengths(
	frequencies: &[usize; 19],
	bitlengths: *mut u32,
) {
	zopfli_length_limited_code_lengths::<7, 19>(frequencies, bitlengths);
}

#[allow(unsafe_code, clippy::cast_sign_loss)]
/// # Length Limited Code Lengths.
///
/// This writes minimum-redundancy length-limited code bitlengths for symbols
/// with the given counts, limited by `MAXBITS`.
///
/// Note that `bitlengths` has the same size as `frequencies` — `SIZE` — but is
/// passed as a pointer instead of an array because `&mut u32` isn't `Copy` and
/// we need to freely pass them around.
pub(crate) fn zopfli_length_limited_code_lengths<const MAXBITS: usize, const SIZE: usize>(
	frequencies: &[usize; SIZE],
	bitlengths: *mut u32,
) {
	// Convert (used) frequencies to leaves. There will never be more than
	// ZOPFLI_NUM_LL of them, but often there will be less, so we'll leverage
	// MaybeUninit to save unnecessary writes.
	let mut leaves: [MaybeUninit<Leaf>; SIZE] = unsafe {
		MaybeUninit::uninit().assume_init()
	};
	let mut len_leaves = 0;
	for (i, &frequency) in frequencies.iter().enumerate() {
		unsafe {
			// Zero out the bitlength regardless.
			let bitlength = bitlengths.add(i);
			bitlength.write(0);

			if frequency != 0 {
				leaves[len_leaves].write(Leaf { frequency, bitlength });
				len_leaves += 1;
			}
		}
	}

	// Nothing to do!
	if len_leaves == 0 { return; }

	// This method is either called with 15 maxbits and 32 or 288 potential
	// leaves, or 7 maxbits and 19 potential leaves; in either case, the max
	// leaves are well within range.
	assert!((1 << MAXBITS) >= len_leaves, "Insufficient maxbits for symbols.");

	// The leaves we can actually use:
	let final_leaves: &mut [Leaf] = unsafe {
		&mut *(std::ptr::addr_of_mut!(leaves[..len_leaves]) as *mut [Leaf])
	};

	// Sortcut: weighting only applies when there are more than two leaves;
	// otherwise we can just record their values as one and call it a day.
	if len_leaves <= 2 {
		for leaf in final_leaves {
			unsafe { leaf.bitlength.write(1); }
		}
		return;
	}

	// Sort the leaves.
	final_leaves.sort();

	// Shrink maxbits if we have fewer leaves. Note that "maxbits" is an
	// inclusive value.
	let maxbits =
		if len_leaves - 1 < MAXBITS { len_leaves - 1 }
		else { MAXBITS };

	// Set up the pool!
	BUMP.with_borrow_mut(|nodes| {
		let lookahead0 = nodes.alloc(Node {
			weight: final_leaves[0].frequency,
			count: 1,
			tail: Cell::new(None),
		});
		let lookahead1 = nodes.alloc(Node {
			weight: final_leaves[1].frequency,
			count: 2,
			tail: Cell::new(None),
		});
		let mut pool = Pool {
			nodes,
			leaves: final_leaves,
		};

		// We won't have more than 15 lists, but we might have fewer.
		let mut lists = [List { lookahead0, lookahead1 }; 15];
		let final_lists = &mut lists[..maxbits];

		// In the last list, (2 * len_leaves - 2) active chains need to be
		// created. We have two already from initialization; each boundary_pm run
		// will give us another.
		let num_boundary_pm_runs = 2 * len_leaves - 4;
		for _ in 0..num_boundary_pm_runs - 1 { pool.boundary_pm(final_lists); }

		// Final touchups!
		pool.boundary_pm_final(final_lists);

		// Write the results!
		pool.write_bit_lengths(final_lists[maxbits - 1].lookahead1);

		// Please be kind, rewind!
		nodes.reset();
	});
}



#[derive(Clone, Copy)]
/// # Leaf.
///
/// This joins a frequency with its matching bitlength from the raw C slices.
struct Leaf {
	frequency: usize,
	bitlength: *mut u32,
}

impl Eq for Leaf {}

impl Ord for Leaf {
	#[inline]
	fn cmp(&self, other: &Self) -> Ordering { self.frequency.cmp(&other.frequency) }
}

impl PartialEq for Leaf {
	#[inline]
	fn eq(&self, other: &Self) -> bool { self.frequency == other.frequency }
}

impl PartialOrd for Leaf {
	#[inline]
	fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}



#[derive(Clone, Copy)]
/// # List.
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



/// # Node Pool.
struct Pool<'a> {
	nodes: &'a Bump,
	leaves: &'a mut [Leaf],
}

impl<'a> Pool<'a> {
	/// # Last Count Frequency.
	const fn last_count_frequency(&self, last_count: usize) -> Option<usize> {
		if last_count < self.leaves.len() {
			Some(self.leaves[last_count].frequency)
		}
		else { None }
	}

	/// # Boundary Package-Merge Step.
	///
	/// Add a new chain to the list, using either a leaf or combination of two
	/// chains from the previous list.
	fn boundary_pm(&mut self, lists: &mut [List<'a>]) {
		let [rest @ .., current] = lists else { unreachable!(); };
		let last_count = current.lookahead1.count;

		// We're at the beginning, which is the end since we're iterating in
		// reverse.
		if rest.is_empty() {
			if let Some(weight) = self.last_count_frequency(last_count) {
				// Shift the lookahead and add a new node.
				current.rotate();
				current.lookahead1 = self.nodes.alloc(Node {
					weight,
					count: last_count + 1,
					tail: current.lookahead0.tail.clone(),
				});
			}
			return;
		}

		// Shift the lookahead.
		current.rotate();

		let previous = rest[rest.len() - 1];
		let weight_sum = previous.weight_sum();

		// Add a leaf and increment the count.
		if let Some(weight) = self.last_count_frequency(last_count) {
			if weight < weight_sum {
				current.lookahead1 = self.nodes.alloc(Node {
					weight,
					count: last_count + 1,
					tail: current.lookahead0.tail.clone(),
				});
				return;
			}
		}

		// Update the tail.
		current.lookahead1 = self.nodes.alloc(Node {
			weight: weight_sum,
			count: last_count,
			tail: Cell::new(Some(previous.lookahead1)),
		});

		// Replace the used-up lookahead chains.
		self.boundary_pm(rest);
		self.boundary_pm(rest);
	}

	/// # Final Package-Merge Step.
	fn boundary_pm_final(&mut self, lists: &mut [List<'a>]) {
		let [_rest @ .., previous, current] = lists else { unreachable!(); };

		let last_count = current.lookahead1.count;
		let weight_sum = previous.weight_sum();
		if last_count < self.leaves.len() && weight_sum > self.leaves[last_count].frequency {
			current.lookahead1 = self.nodes.alloc(Node {
				weight: 0,
				count: last_count + 1,
				tail: current.lookahead1.tail.clone(),
			});
		}
		else {
			current.lookahead1.tail.set(Some(previous.lookahead1));
		}
	}

	#[allow(unsafe_code)]
	/// # Extract/Write Bit Lengths.
	///
	/// Copy the bit lengths from the last chain of the last list.
	fn write_bit_lengths(&mut self, mut node: &'a Node<'a>) {
		let mut val = node.count;
		let mut value = 1;
		while let Some(tail) = node.tail.get() {
			if val > tail.count {
				for leaf in &mut self.leaves[tail.count..val] {
					unsafe { leaf.bitlength.write(value); }
				}
				val = tail.count;
			}
			value += 1;
			node = tail;
		}
		for leaf in &mut self.leaves[..val] {
			unsafe { leaf.bitlength.write(value); }
		}
	}
}
