#include "custom_png_deflate.h"

unsigned custom_png_deflate(unsigned char** out, size_t* outsize,
                          const unsigned char* in, size_t insize,
                          const LodePNGCompressSettings* settings) {
  unsigned char bp = 0;
  ZopfliOptions options;
  ZopfliInitOptions(&options);
  options.numiterations = flaca_zopfli_iterations(insize);

  ZopfliDeflate(&options, 2, 1, in, insize, &bp, out, outsize);
  return 0;
}
