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
NOTE: The actual zopfli functionality has been entirely rewritten in Rust. All
that remains of the original project is the C-to-C bit-writing/malloc stuff,
gathered here for convenience.
*/

#ifndef ZOPFLI_H_
#define ZOPFLI_H_

#include <stdlib.h>

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
Appends value to dynamically allocated memory, doubling its allocation size
whenever needed.

value: the value to append, type T
data: pointer to the dynamic array to append to, type T**
size: pointer to the size of the array to append to, type size_t*. This is the
size that you consider the array to be, not the internal allocation size.
Precondition: allocated size of data is at least a power of two greater than or
equal than *size.
*/
#define ZOPFLI_APPEND_DATA(/* T */ value, /* T** */ data, /* size_t* */ size) {\
	if (!((*size) & ((*size) - 1))) {\
		/*double alloc size if it's a power of two*/\
		(*data) = (*size) == 0 ? malloc(sizeof(**data))\
			: realloc((*data), (*size) * 2 * sizeof(**data));\
	}\
	(*data)[(*size)] = (value);\
	(*size)++;\
}

#endif  /* ZOPFLI_H_ */
