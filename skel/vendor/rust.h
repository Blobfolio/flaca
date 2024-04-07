/*
Rust Externs.

Since Google has effectively abandoned Zopfli — shocker! — we're moving most of
the methods into Rust where they can be more easily managed. At some point
hopefully all of it will live Rust side, but until then, these headers allow
interop with the remaining sea of C.
*/

#ifndef ZOPFLI_RUST_H_
#define ZOPFLI_RUST_H_

#include <stdlib.h> /* for size_t */
#include "lodepng/lodepng.h"

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
void ZopfliLengthLimitedCodeLengths(const size_t* frequencies, unsigned* bitlengths);

/*
Code Lengths to Symbols.

Converts a series of Huffman tree bitlengths, to the bit values of the symbols.

The original method was split into two to improve memory allocations on the
Rust side.
*/
void ZopfliLengthsToSymbolsCode(const unsigned* lengths, unsigned* symbols);

#endif
