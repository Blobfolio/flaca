#include "rust.h"
#include "lodepng/lodepng.h"
#include "zopfli/deflate.h"

/*
Custom PNG Deflate

This uses Zopfli for compression.
*/
unsigned custom_png_deflate(unsigned char** out, size_t* outsize,
                          const unsigned char* in, size_t insize,
                          const LodePNGCompressSettings* settings);
