/*
Rust Externs.

Since Google has effectively abandoned Zopfli — shocker! — we're moving some of
the methods into Rust where they can be more easily managed.

This file contains the signatures to allow (more or less) seamless
interop across the sea of C.
*/

#include <stdlib.h> /* for size_t */

/*
Zopfli Iterations.

This returns the desired number of iterations to use during compression. (This
is used by our custom_png_deflate callback; it is not part of the original
code base.)
*/
int flaca_zopfli_iterations(size_t);

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

/* Extracts sublen array from the cache. */
void ZopfliCacheToSublen(size_t pos, size_t length, unsigned short* sublen);

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

/* Returns the length up to which could be stored in the cache. */
unsigned ZopfliMaxCachedSublen(size_t pos);

/* Stores sublen array in the cache. */
void ZopfliSublenToCache(const unsigned short* sublen, size_t pos, size_t length);
