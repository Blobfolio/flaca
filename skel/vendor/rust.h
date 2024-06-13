/*
Rust Externs.

This exposes the header for our custom lodepng deflater (i.e. zopfli).
*/

#ifndef ZOPFLI_RUST_H_
#define ZOPFLI_RUST_H_

#include <stdlib.h> /* for size_t */
#include "lodepng/lodepng.h"

/*
Custom (Zopfli) Deflate Callback.
*/
unsigned flaca_png_deflate(
	unsigned char** out, size_t* outsize,
	const unsigned char* in, size_t insize,
	const LodePNGCompressSettings* settings);

#endif
