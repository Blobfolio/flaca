/*
Rust Externs.

Since Google has effectively abandoned Zopfli — shocker! — we're moving some of
the methods into Rust where they can be more easily managed.

This file contains the extern signatures to allow (more or less) seamless
interop across the sea of C.
*/

#include <stdlib.h> /* for size_t */

/*
Zopfli Iterations.

This returns the desired number of iterations to use during compression. (This
is used by our custom_png_deflate callback; it is not part of the original
code base.)
*/
extern int flaca_zopfli_iterations(size_t);

/*
Write Fixed Tree.

Initialize the length and distance symbol arrays with fixed tree values.
*/
void GetFixedTree(unsigned* ll_lengths, unsigned* d_lengths);

/*
Code Lengths to Symbols.

Converts a series of Huffman tree bitlengths, to the bit values of the symbols.

The original method was split into two to improve memory allocations on the
Rust side.
*/
extern void ZopfliLengthsToSymbols7(const unsigned* lengths, size_t n, unsigned* symbols);
extern void ZopfliLengthsToSymbols15(const unsigned* lengths, size_t n, unsigned* symbols);

/*
Calculate Symbol Entropy.

Calculates the entropy of each symbol, based on the counts of each symbol. The
result is similar to the result of ZopfliCalculateBitLengths, but with the
actual theoritical bit lengths according to the entropy. Since the resulting
values are fractional, they cannot be used to encode the tree specified by
DEFLATE.
*/
extern void ZopfliCalculateEntropy(const size_t* count, size_t n, double* bitlengths);
