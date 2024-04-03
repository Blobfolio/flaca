/*
Rust Externs.

Since Google has effectively abandoned Zopfli — shocker! — we're moving some of
the methods into Rust where they can be more easily managed.

This file contains the signatures to allow (more or less) seamless
interop across the sea of C.
*/

#ifndef ZOPFLI_RUST_H_
#define ZOPFLI_RUST_H_

#include <stdlib.h> /* for size_t */
#include "lodepng/lodepng.h"
#include "zopfli/util.h"


/* Not ours, just moved. */
typedef struct ZopfliLZ77Store {
	unsigned short* litlens;  /* Lit or len. */
	unsigned short* dists;  /* If 0: indicates literal in corresponding litlens,
			if > 0: length in corresponding litlens, this is the distance. */
	size_t size;

	const unsigned char* data;  /* original data */
	size_t* pos;  /* position in data where this LZ77 command begins */

	unsigned short* ll_symbol;
	unsigned short* d_symbol;

	/* Cumulative histograms wrapping around per chunk. Each chunk has the amount
	of distinct symbols as length, so using 1 value per LZ77 symbol, we have a
	precise histogram at every N symbols, and the rest can be calculated by
	looping through the actual symbols of this chunk. */
	size_t* ll_counts;
	size_t* d_counts;
} ZopfliLZ77Store;



/*
Custom Deflate Callback.

This tells LodePNG to use Zopfli.
*/
unsigned flaca_png_deflate(
	unsigned char** out, size_t* outsize,
	const unsigned char* in, size_t insize,
	const LodePNGCompressSettings* settings);

/*
Write Fixed Tree.

Initialize the length and distance symbol arrays with fixed tree values.
*/
void GetFixedTree(unsigned* ll_lengths, unsigned* d_lengths);

/*
Optimize Huffman RLE Compression.

Changes the population counts in a way that the consequent Huffman tree
compression, especially its rle-part, will be more likely to compress this data
more efficiently. length contains the size of the histogram.
*/
void OptimizeHuffmanForRle(size_t length, size_t* counts);

/*
Patch Buggy Distance Codes.

Ensures there are at least 2 distance codes to support buggy decoders.
Zlib 1.2.1 and below have a bug where it fails if there isn't at least 1
distance code (with length > 0), even though it's valid according to the
deflate spec to have 0 distance codes. On top of that, some mobile phones
require at least two distance codes. To support these decoders too (but
potentially at the cost of a few bytes), add dummy code lengths of 1.
References to this bug can be found in the changelog of
Zlib 1.2.2 and here: http://www.jonof.id.au/forum/index.php?topic=515.0.

d_lengths: the 32 lengths of the distance codes.
*/
void PatchDistanceCodesForBuggyDecoders(unsigned* d_lengths);

/*
Length Limited Code Lengths.

Outputs minimum-redundancy length-limited code bitlengths for symbols with the
given counts. The bitlengths are limited by maxbits.

The output is tailored for DEFLATE: symbols that never occur, get a bit length
of 0, and if only a single symbol occurs at least once, its bitlength will be 1,
and not 0 as would theoretically be needed for a single symbol.

frequencies: The amount of occurrences of each symbol.
n: The amount of symbols.
maxbits: Maximum bit length, inclusive.
bitlengths: Output, the bitlengths for the symbol prefix codes.
return: 0 for OK, non-0 for error.
*/
void ZopfliLengthLimitedCodeLengths(
	const size_t* frequencies, size_t n, size_t maxbits, unsigned* bitlengths);

/*
Code Lengths to Symbols.

Converts a series of Huffman tree bitlengths, to the bit values of the symbols.

The original method was split into two to improve memory allocations on the
Rust side.
*/
void ZopfliLengthsToSymbols7(const unsigned* lengths, size_t n, unsigned* symbols);
void ZopfliLengthsToSymbols15(const unsigned* lengths, size_t n, unsigned* symbols);

/*
Does LZ77 using an algorithm similar to gzip, with lazy matching, rather than
with the slow but better "squeeze" implementation.
The result is placed in the ZopfliLZ77Store.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77Greedy(
	size_t cache, const unsigned char* in, size_t instart, size_t inend,
	ZopfliLZ77Store* store);

/*
Calculates lit/len and dist pairs for given data.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77Optimal(
	const unsigned char* in, size_t instart, size_t inend,
	int numiterations, ZopfliLZ77Store* store);

/*
Does the same as ZopfliLZ77Optimal, but optimized for the fixed tree of the
deflate standard.
The fixed tree never gives the best compression. But this gives the best
possible LZ77 encoding possible with the fixed tree.
This does not create or output any fixed tree, only LZ77 data optimized for
using with a fixed tree.
If instart is larger than 0, it uses values before instart as starting
dictionary.
*/
void ZopfliLZ77OptimalFixed(
	const unsigned char* in, size_t instart, size_t inend, ZopfliLZ77Store* store);

#endif
