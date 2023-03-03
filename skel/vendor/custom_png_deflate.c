#include "custom_png_deflate.h"

unsigned custom_png_deflate(unsigned char** out, size_t* outsize,
                          const unsigned char* in, size_t insize,
                          const LodePNGCompressSettings* settings) {
  unsigned char bp = 0;
  int numiterations = *(int*)settings->custom_context;
  ZopfliOptions options;
  ZopfliInitOptions(&options);

  if (numiterations <= 0) {
  	options.numiterations = insize < 200000 ? 60 : 20;
  }
  else {
  	options.numiterations = numiterations;
  }

  ZopfliDeflate(&options, 2, 1, in, insize, &bp, out, outsize);
  return 0;
}
