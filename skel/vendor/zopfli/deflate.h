/*
Copyright 2011 Google Inc. All Rights Reserved.

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

Author: lode.vandevenne@gmail.com (Lode Vandevenne)
Author: jyrki.alakuijala@gmail.com (Jyrki Alakuijala)
*/

#ifndef ZOPFLI_DEFLATE_H_
#define ZOPFLI_DEFLATE_H_

/*
Functions to compress according to the DEFLATE specification, using the
"squeeze" LZ77 compression backend.
*/

#include "lz77.h"

/*
Like ZopfliDeflate, but allows to specify start and end byte with instart and
inend. Only that part is compressed, but earlier bytes are still used for the
back window.
*/
void ZopfliDeflatePart(
	int numiterations, int final,
	const unsigned char* in, size_t instart, size_t inend,
	unsigned char* bp, unsigned char** out, size_t* outsize);

/*
Calculates block size in bits.
litlens: lz77 lit/lengths
dists: ll77 distances
lstart: start of block
lend: end of block (not inclusive)
*/
double ZopfliCalculateBlockSize(
	const ZopfliLZ77Store* lz77, size_t lstart, size_t lend, int btype);

/*
Calculates block size in bits, automatically using the best btype.
*/
double ZopfliCalculateBlockSizeAutoType(
	const ZopfliLZ77Store* lz77, size_t lstart, size_t lend);

#endif  /* ZOPFLI_DEFLATE_H_ */
