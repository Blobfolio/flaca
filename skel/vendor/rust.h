/*
Rust Externs.

Since Google has effectively abandoned Zopfli — shocker! — we're moving some of
the methods into Rust where they can be more easily managed.

This file contains the signatures to allow (more or less) seamless
interop across the sea of C.
*/

#include <stdlib.h> /* for size_t */
#include "lodepng/lodepng.h"

/*
Custom Deflate Callback.

This tells LodePNG to use Zopfli.
*/
unsigned flaca_png_deflate(unsigned char** out, size_t* outsize,
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
void OptimizeHuffmanForRle(int length, size_t* counts);

/*
Maybe Fetch Length/Distance/Sublength from Cache.

Gets distance, length and sublen values from the cache if possible.
Returns 1 if it got the values from the cache, 0 if not.
Updates the limit value to a smaller one if possible with more limited
information from the cache.
*/
int TryGetFromLongestMatchCache(size_t lmcpos, size_t* limit,
    unsigned short* sublen, unsigned short* distance, unsigned short* length);

/*
Calculate Symbol Entropy.

Calculates the entropy of each symbol, based on the counts of each symbol. The
result is similar to the result of ZopfliCalculateBitLengths, but with the
actual theoritical bit lengths according to the entropy. Since the resulting
values are fractional, they cannot be used to encode the tree specified by
DEFLATE.
*/
void ZopfliCalculateEntropy(const size_t* count, size_t n, double* bitlengths);

/* Initializes the ZopfliLongestMatchCache. */
void ZopfliInitCache(size_t blocksize);

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
    const size_t* frequencies, int n, int maxbits, unsigned* bitlengths);

/*
Code Lengths to Symbols.

Converts a series of Huffman tree bitlengths, to the bit values of the symbols.

The original method was split into two to improve memory allocations on the
Rust side.
*/
void ZopfliLengthsToSymbols7(const unsigned* lengths, size_t n, unsigned* symbols);
void ZopfliLengthsToSymbols15(const unsigned* lengths, size_t n, unsigned* symbols);

/* Fetch sublength length and distance from cache. */
unsigned short ZopfliLongestMatchCacheLD(size_t pos, unsigned short* len, unsigned short* dist);

/* Set cached length and distance. */
void ZopfliLongestMatchCacheSetLD(size_t pos, unsigned short len, unsigned short dist);

/* Stores sublen array in the cache. */
void ZopfliSublenToCache(const unsigned short* sublen, size_t pos, size_t length);
