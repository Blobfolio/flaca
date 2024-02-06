#include "lodepng/lodepng.h"
#include "zopfli/deflate.h"

/*
Rust Extern: Zopfli Iterations

This returns the number of iterations to use during compression.
*/
extern int flaca_zopfli_iterations(size_t);

/*
Custom PNG Deflate

This uses Zopfli for compression.
*/
unsigned custom_png_deflate(unsigned char** out, size_t* outsize,
                          const unsigned char* in, size_t insize,
                          const LodePNGCompressSettings* settings);
