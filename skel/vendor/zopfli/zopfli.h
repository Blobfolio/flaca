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

/*
Note: almost all of the (relevant) zopfli code has been rewritten and ported to
the Rust side of Flaca; this file contains all that remains, i.e. the
bit-related, malloc-calling stuff. (There's no point managing C memory from
Rust.)
*/

#ifndef ZOPFLI_H_
#define ZOPFLI_H_

#include <stdlib.h>
#include "../rust.h"

/*
bp = bitpointer, always in range [0, 7].
The outsize is number of necessary bytes to encode the bits.
Given the value of bp and the amount of bytes, the amount of bits represented
is not simply bytesize * 8 + bp because even representing one bit requires a
whole byte. It is: (bp == 0) ? (bytesize * 8) : ((bytesize - 1) * 8 + bp)
*/
void ZopfliAddBit(int bit, unsigned char* bp, unsigned char** out, size_t* outsize);

void ZopfliAddBits(
	unsigned symbol, unsigned length,
	unsigned char* bp, unsigned char** out, size_t* outsize);

/*
Adds bits, like AddBits, but the order is inverted. The deflate specification
uses both orders in one standard.
*/
void ZopfliAddHuffmanBits(
	unsigned symbol, unsigned length,
	unsigned char* bp, unsigned char** out, size_t* outsize);

/* Since an uncompressed block can be max 65535 in size, it actually adds
multible blocks if needed. */
void ZopfliAddNonCompressedBlock(
	int final, const unsigned char* in, size_t instart, size_t inend,
	unsigned char* bp, unsigned char** out, size_t* outsize);

/*
Encodes the Huffman tree and returns how many bits its encoding takes. If out
is a null pointer, only returns the size and runs faster.
*/
size_t ZopfliEncodeTree(
	const unsigned* ll_lengths, const unsigned* d_lengths,
	int use_16, int use_17, int use_18,
	unsigned char* bp, unsigned char** out, size_t* outsize);

#endif  /* ZOPFLI_H_ */
