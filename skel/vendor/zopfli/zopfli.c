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

#include "zopfli.h"

#include <stdio.h>
#include <stdlib.h>

/*
bp = bitpointer, always in range [0, 7].
The outsize is number of necessary bytes to encode the bits.
Given the value of bp and the amount of bytes, the amount of bits represented
is not simply bytesize * 8 + bp because even representing one bit requires a
whole byte. It is: (bp == 0) ? (bytesize * 8) : ((bytesize - 1) * 8 + bp)
*/
void ZopfliAddBit(
	int bit, unsigned char* bp, unsigned char** out, size_t* outsize) {
	if (*bp == 0) ZOPFLI_APPEND_DATA(0, out, outsize);
	(*out)[*outsize - 1] |= bit << *bp;
	*bp = (*bp + 1) & 7;
}

void ZopfliAddBits(
	unsigned symbol, unsigned length,
	unsigned char* bp, unsigned char** out, size_t* outsize) {
	/* TODO(lode): make more efficient (add more bits at once). */
	unsigned i;
	for (i = 0; i < length; i++) {
		unsigned bit = (symbol >> i) & 1;
		if (*bp == 0) ZOPFLI_APPEND_DATA(0, out, outsize);
		(*out)[*outsize - 1] |= bit << *bp;
		*bp = (*bp + 1) & 7;
	}
}

/*
Adds bits, like ZopfliAddBits, but the order is inverted. The deflate specification
uses both orders in one standard.
*/
void ZopfliAddHuffmanBits(
	unsigned symbol, unsigned length,
	unsigned char* bp, unsigned char** out, size_t* outsize) {
	/* TODO(lode): make more efficient (add more bits at once). */
	unsigned i;
	for (i = 0; i < length; i++) {
		unsigned bit = (symbol >> (length - i - 1)) & 1;
		if (*bp == 0) ZOPFLI_APPEND_DATA(0, out, outsize);
		(*out)[*outsize - 1] |= bit << *bp;
		*bp = (*bp + 1) & 7;
	}
}

/* Since an uncompressed block can be max 65535 in size, it actually adds
multible blocks if needed. */
void ZopfliAddNonCompressedBlock(
	int final, const unsigned char* in, size_t instart, size_t inend,
	unsigned char* bp, unsigned char** out, size_t* outsize) {
	size_t pos = instart;
	for (;;) {
		size_t i;
		unsigned short blocksize = 65535;
		unsigned short nlen;
		int currentfinal;

		if (pos + blocksize > inend) blocksize = inend - pos;
		currentfinal = pos + blocksize >= inend;

		nlen = ~blocksize;

		ZopfliAddBit(final && currentfinal, bp, out, outsize);
		/* BTYPE 00 */
		ZopfliAddBit(0, bp, out, outsize);
		ZopfliAddBit(0, bp, out, outsize);

		/* Any bits of input up to the next byte boundary are ignored. */
		*bp = 0;

		ZOPFLI_APPEND_DATA(blocksize % 256, out, outsize);
		ZOPFLI_APPEND_DATA((blocksize / 256) % 256, out, outsize);
		ZOPFLI_APPEND_DATA(nlen % 256, out, outsize);
		ZOPFLI_APPEND_DATA((nlen / 256) % 256, out, outsize);

		for (i = 0; i < blocksize; i++) {
			ZOPFLI_APPEND_DATA(in[pos + i], out, outsize);
		}

		if (currentfinal) break;
		pos += blocksize;
	}
}
