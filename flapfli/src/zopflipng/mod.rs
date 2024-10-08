/*!
# Flapfli: Zopflipng!

The `optimize` method in this module emulates the behaviors of the zopflipng
CLI tool when called with:

```bash
zopflipng -m <input> <output>
```

As Google is no longer maintaining the original zopfli project, all relevant
functionality supporting the above has been rewritten and ported to Rust,
resulting in code that is safer, (slightly) saner, and ultimately more
performant.
*/

mod blocks;
mod cache;
mod chunk;
mod error;
mod hash;
mod iter;
mod kat;
mod lz77;
mod rle;
mod rng;
mod stats;
mod symbols;

pub(crate) use blocks::deflate_part;
use cache::{
	MatchCache,
	SplitCache,
	SqueezeCache,
};
pub(crate) use chunk::ZopfliChunk;
use error::{
	zopfli_error,
	ZopfliError,
};
pub(crate) use hash::ZopfliState;
use iter::ReducingSlices;
use kat::{
	best_tree_size,
	encode_tree,
	LengthLimitedCodeLengths,
};
use lz77::{
	LZ77Store,
	LZ77StoreRange,
};
use rng::ZopfliRange;
use rle::DynamicLengths;
use super::deflate::ZopfliOut;
use symbols::{
	DeflateSym,
	DeflateSymBasic,
	DISTANCE_BITS,
	DISTANCE_BITS_F,
	DISTANCE_SYMBOLS,
	DISTANCE_VALUES,
	Dsym,
	LENGTH_SYMBOL_BIT_VALUES,
	LENGTH_SYMBOL_BITS,
	LENGTH_SYMBOL_BITS_F,
	LENGTH_SYMBOLS,
	LitLen,
	Lsym,
	SplitLen,
	TreeDist,
};



/// # Size of Litlen Collections.
const ZOPFLI_NUM_LL: usize = 288;

/// # Size of Distance Collections.
const ZOPFLI_NUM_D: usize = 32;

/// # Zero-Filled Distance Counts.
const ZEROED_COUNTS_D: ArrayD<u32> = [0; ZOPFLI_NUM_D];

/// # Zero-Filled Litlen Counts.
const ZEROED_COUNTS_LL: ArrayLL<u32> = [0; ZOPFLI_NUM_LL];

/// # Fixed Litlen Tree.
const FIXED_TREE_LL: ArrayLL<DeflateSym> = [
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
	DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09, DeflateSym::D09,
	DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D07, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08, DeflateSym::D08,
];

/// # Fixed Litlen Symbols.
const FIXED_SYMBOLS_LL: ArrayLL<u32> = [
	48, 49, 50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71,
	72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95,
	96, 97, 98, 99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119,
	120, 121, 122, 123, 124, 125, 126, 127, 128, 129, 130, 131, 132, 133, 134, 135, 136, 137, 138, 139, 140, 141, 142, 143,
	144, 145, 146, 147, 148, 149, 150, 151, 152, 153, 154, 155, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167,
	168, 169, 170, 171, 172, 173, 174, 175, 176, 177, 178, 179, 180, 181, 182, 183, 184, 185, 186, 187, 188, 189, 190, 191,
	400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416, 417, 418, 419, 420, 421, 422, 423,
	424, 425, 426, 427, 428, 429, 430, 431, 432, 433, 434, 435, 436, 437, 438, 439, 440, 441, 442, 443, 444, 445, 446, 447,
	448, 449, 450, 451, 452, 453, 454, 455, 456, 457, 458, 459, 460, 461, 462, 463, 464, 465, 466, 467, 468, 469, 470, 471,
	472, 473, 474, 475, 476, 477, 478, 479, 480, 481, 482, 483, 484, 485, 486, 487, 488, 489, 490, 491, 492, 493, 494, 495,
	496, 497, 498, 499, 500, 501, 502, 503, 504, 505, 506, 507, 508, 509, 510, 511, 0, 1, 2, 3, 4, 5, 6, 7,
	8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 192, 193, 194, 195, 196, 197, 198, 199
];

/// # Fixed Distance Tree.
const FIXED_TREE_D: ArrayD<DeflateSym> = [DeflateSym::D05; ZOPFLI_NUM_D];

/// # Fixed Distance Symbols.
const FIXED_SYMBOLS_D: ArrayD<u32> = [
	0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
	16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
];

/// # Step Size for Deflate Parts.
///
/// The "active" portion of the `ZopfliChunk` passed from lodepng will never
/// exceed a million bytes.
pub(super) const ZOPFLI_MASTER_BLOCK_SIZE: usize = 1_000_000;

/// # Hash/LZ77 Window Size.
///
/// This is the window size used by lodepng when zopfli processing is enabled,
/// and the amount expected by structs like `ZopfliHash`.
const ZOPFLI_WINDOW_SIZE: usize = 32_768;

/// # Minimum Matchable Distance.
const ZOPFLI_MIN_MATCH: usize = 3;

/// # Maximum Matchable Distance.
const ZOPFLI_MAX_MATCH: usize = 258;

/// # Length of Sublength Array.
///
/// The squeeze sublength array slices have indices spanning
/// `0..=ZOPFLI_MAX_MATCH`.
const SUBLEN_LEN: usize = ZOPFLI_MAX_MATCH + 1;

/// # Array with `ZOPFLI_NUM_LL` Entries.
type ArrayLL<T> = [T; ZOPFLI_NUM_LL];

/// # Array with `ZOPFLI_NUM_D` Entries.
type ArrayD<T> = [T; ZOPFLI_NUM_D];
