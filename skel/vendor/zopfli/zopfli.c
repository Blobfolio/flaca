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
#include "util.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

/* __has_builtin available in clang */
#ifdef __has_builtin
# if __has_builtin(__builtin_clz)
#   define ZOPFLI_HAS_BUILTIN_CLZ
# endif
/* __builtin_clz available beginning with GCC 3.4 */
#elif __GNUC__ * 100 + __GNUC_MINOR__ >= 304
# define ZOPFLI_HAS_BUILTIN_CLZ
#endif

/* Gets the symbol for the given dist, cfr. the DEFLATE spec. */
static int ZopfliGetDistSymbol(int dist) {
#ifdef ZOPFLI_HAS_BUILTIN_CLZ
	if (dist < 5) {
		return dist - 1;
	} else {
		int l = (31 ^ __builtin_clz(dist - 1)); /* log2(dist - 1) */
		int r = ((dist - 1) >> (l - 1)) & 1;
		return l * 2 + r;
	}
#else
	if (dist < 193) {
		if (dist < 13) {  /* dist 0..13. */
			if (dist < 5) return dist - 1;
			else if (dist < 7) return 4;
			else if (dist < 9) return 5;
			else return 6;
		} else {  /* dist 13..193. */
			if (dist < 17) return 7;
			else if (dist < 25) return 8;
			else if (dist < 33) return 9;
			else if (dist < 49) return 10;
			else if (dist < 65) return 11;
			else if (dist < 97) return 12;
			else if (dist < 129) return 13;
			else return 14;
		}
	} else {
		if (dist < 2049) {  /* dist 193..2049. */
			if (dist < 257) return 15;
			else if (dist < 385) return 16;
			else if (dist < 513) return 17;
			else if (dist < 769) return 18;
			else if (dist < 1025) return 19;
			else if (dist < 1537) return 20;
			else return 21;
		} else {  /* dist 2049..32768. */
			if (dist < 3073) return 22;
			else if (dist < 4097) return 23;
			else if (dist < 6145) return 24;
			else if (dist < 8193) return 25;
			else if (dist < 12289) return 26;
			else if (dist < 16385) return 27;
			else if (dist < 24577) return 28;
			else return 29;
		}
	}
#endif
}

/*
Gets the symbol for the given length, cfr. the DEFLATE spec.
Returns the symbol in the range [257-285] (inclusive)
*/
static int ZopfliGetLengthSymbol(int l) {
	static const int table[259] = {
		0, 0, 0, 257, 258, 259, 260, 261, 262, 263, 264,
		265, 265, 266, 266, 267, 267, 268, 268,
		269, 269, 269, 269, 270, 270, 270, 270,
		271, 271, 271, 271, 272, 272, 272, 272,
		273, 273, 273, 273, 273, 273, 273, 273,
		274, 274, 274, 274, 274, 274, 274, 274,
		275, 275, 275, 275, 275, 275, 275, 275,
		276, 276, 276, 276, 276, 276, 276, 276,
		277, 277, 277, 277, 277, 277, 277, 277,
		277, 277, 277, 277, 277, 277, 277, 277,
		278, 278, 278, 278, 278, 278, 278, 278,
		278, 278, 278, 278, 278, 278, 278, 278,
		279, 279, 279, 279, 279, 279, 279, 279,
		279, 279, 279, 279, 279, 279, 279, 279,
		280, 280, 280, 280, 280, 280, 280, 280,
		280, 280, 280, 280, 280, 280, 280, 280,
		281, 281, 281, 281, 281, 281, 281, 281,
		281, 281, 281, 281, 281, 281, 281, 281,
		281, 281, 281, 281, 281, 281, 281, 281,
		281, 281, 281, 281, 281, 281, 281, 281,
		282, 282, 282, 282, 282, 282, 282, 282,
		282, 282, 282, 282, 282, 282, 282, 282,
		282, 282, 282, 282, 282, 282, 282, 282,
		282, 282, 282, 282, 282, 282, 282, 282,
		283, 283, 283, 283, 283, 283, 283, 283,
		283, 283, 283, 283, 283, 283, 283, 283,
		283, 283, 283, 283, 283, 283, 283, 283,
		283, 283, 283, 283, 283, 283, 283, 283,
		284, 284, 284, 284, 284, 284, 284, 284,
		284, 284, 284, 284, 284, 284, 284, 284,
		284, 284, 284, 284, 284, 284, 284, 284,
		284, 284, 284, 284, 284, 284, 284, 285
	};
	return table[l];
}

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

/*
Encodes the Huffman tree and returns how many bits its encoding takes. If out
is a null pointer, only returns the size and runs faster.
*/
size_t ZopfliEncodeTree(
	const unsigned* ll_lengths, const unsigned* d_lengths,
	int use_16, int use_17, int use_18,
	unsigned char* bp, unsigned char** out, size_t* outsize) {
	unsigned lld_total;  /* Total amount of literal, length, distance codes. */
	/* Runlength encoded version of lengths of litlen and dist trees. */
	unsigned* rle = 0;
	unsigned* rle_bits = 0;  /* Extra bits for rle values 16, 17 and 18. */
	size_t rle_size = 0;  /* Size of rle array. */
	size_t rle_bits_size = 0;  /* Should have same value as rle_size. */
	unsigned hlit = 29;  /* 286 - 257 */
	unsigned hdist = 29;  /* 32 - 1, but gzip does not like hdist > 29.*/
	unsigned hclen;
	unsigned hlit2;
	size_t i, j;
	size_t clcounts[19];
	unsigned clcl[19];  /* Code length code lengths. */
	unsigned clsymbols[19];
	/* The order in which code length code lengths are encoded as per deflate. */
	static const unsigned order[19] = {
		16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15
	};
	int size_only = !out;
	size_t result_size = 0;

	for (i = 0; i < 19; i++) clcounts[i] = 0;

	/* Trim zeros. */
	while (hlit > 0 && ll_lengths[257 + hlit - 1] == 0) hlit--;
	while (hdist > 0 && d_lengths[1 + hdist - 1] == 0) hdist--;
	hlit2 = hlit + 257;

	lld_total = hlit2 + hdist + 1;

	for (i = 0; i < lld_total; i++) {
		/* This is an encoding of a huffman tree, so now the length is a symbol */
		unsigned char symbol = i < hlit2 ? ll_lengths[i] : d_lengths[i - hlit2];
		unsigned count = 1;
		if(use_16 || (symbol == 0 && (use_17 || use_18))) {
			for (j = i + 1; j < lld_total && symbol ==
					(j < hlit2 ? ll_lengths[j] : d_lengths[j - hlit2]); j++) {
				count++;
			}
		}
		i += count - 1;

		/* Repetitions of zeroes */
		if (symbol == 0 && count >= 3) {
			if (use_18) {
				while (count >= 11) {
					unsigned count2 = count > 138 ? 138 : count;
					if (!size_only) {
						ZOPFLI_APPEND_DATA(18, &rle, &rle_size);
						ZOPFLI_APPEND_DATA(count2 - 11, &rle_bits, &rle_bits_size);
					}
					clcounts[18]++;
					count -= count2;
				}
			}
			if (use_17) {
				while (count >= 3) {
					unsigned count2 = count > 10 ? 10 : count;
					if (!size_only) {
						ZOPFLI_APPEND_DATA(17, &rle, &rle_size);
						ZOPFLI_APPEND_DATA(count2 - 3, &rle_bits, &rle_bits_size);
					}
					clcounts[17]++;
					count -= count2;
				}
			}
		}

		/* Repetitions of any symbol */
		if (use_16 && count >= 4) {
			count--;  /* Since the first one is hardcoded. */
			clcounts[symbol]++;
			if (!size_only) {
				ZOPFLI_APPEND_DATA(symbol, &rle, &rle_size);
				ZOPFLI_APPEND_DATA(0, &rle_bits, &rle_bits_size);
			}
			while (count >= 3) {
				unsigned count2 = count > 6 ? 6 : count;
				if (!size_only) {
					ZOPFLI_APPEND_DATA(16, &rle, &rle_size);
					ZOPFLI_APPEND_DATA(count2 - 3, &rle_bits, &rle_bits_size);
				}
				clcounts[16]++;
				count -= count2;
			}
		}

		/* No or insufficient repetition */
		clcounts[symbol] += count;
		while (count > 0) {
			if (!size_only) {
				ZOPFLI_APPEND_DATA(symbol, &rle, &rle_size);
				ZOPFLI_APPEND_DATA(0, &rle_bits, &rle_bits_size);
			}
			count--;
		}
	}

	ZopfliLengthLimitedCodeLengths(clcounts, clcl);
	if (!size_only) ZopfliLengthsToSymbolsCode(clcl, clsymbols);

	hclen = 15;
	/* Trim zeros. */
	while (hclen > 0 && clcounts[order[hclen + 4 - 1]] == 0) hclen--;

	if (!size_only) {
		ZopfliAddBits(hlit, 5, bp, out, outsize);
		ZopfliAddBits(hdist, 5, bp, out, outsize);
		ZopfliAddBits(hclen, 4, bp, out, outsize);

		for (i = 0; i < hclen + 4; i++) {
			ZopfliAddBits(clcl[order[i]], 3, bp, out, outsize);
		}

		for (i = 0; i < rle_size; i++) {
			unsigned symbol = clsymbols[rle[i]];
			ZopfliAddHuffmanBits(symbol, clcl[rle[i]], bp, out, outsize);
			/* Extra bits. */
			if (rle[i] == 16) ZopfliAddBits(rle_bits[i], 2, bp, out, outsize);
			else if (rle[i] == 17) ZopfliAddBits(rle_bits[i], 3, bp, out, outsize);
			else if (rle[i] == 18) ZopfliAddBits(rle_bits[i], 7, bp, out, outsize);
		}
	}

	result_size += 14;  /* hlit, hdist, hclen bits */
	result_size += (hclen + 4) * 3;  /* clcl bits */
	for(i = 0; i < 19; i++) {
		result_size += clcl[i] * clcounts[i];
	}
	/* Extra bits. */
	result_size += clcounts[16] * 2;
	result_size += clcounts[17] * 3;
	result_size += clcounts[18] * 7;

	/* Note: in case of "size_only" these are null pointers so no effect. */
	free(rle);
	free(rle_bits);

	return result_size;
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

void ZopfliInitLZ77Store(const unsigned char* data, ZopfliLZ77Store* store) {
	store->size = 0;
	store->litlens = 0;
	store->dists = 0;
	store->pos = 0;
	store->data = data;
	store->ll_symbol = 0;
	store->d_symbol = 0;
	store->ll_counts = 0;
	store->d_counts = 0;
}

void ZopfliCleanLZ77Store(ZopfliLZ77Store* store) {
	free(store->litlens);
	free(store->dists);
	free(store->pos);
	free(store->ll_symbol);
	free(store->d_symbol);
	free(store->ll_counts);
	free(store->d_counts);
}

void ZopfliReInitLZ77Store(const unsigned char* data, ZopfliLZ77Store* store, size_t size, size_t llsize, size_t dsize) {
	store->data = data;
	if (store->size == size) return;

	store->size = size;
	store->litlens = (unsigned short*)realloc(store->litlens, sizeof(*store->litlens) * size);
	store->dists = (unsigned short*)realloc(store->dists, sizeof(*store->dists) * size);
	store->pos = (size_t*)realloc(store->pos, sizeof(*store->pos) * size);
	store->ll_symbol = (unsigned short*)realloc(store->ll_symbol, sizeof(*store->ll_symbol) * size);
	store->d_symbol = (unsigned short*)realloc(store->d_symbol, sizeof(*store->d_symbol) * size);
	store->ll_counts = (size_t*)realloc(store->ll_counts, sizeof(*store->ll_counts) * llsize);
	store->d_counts = (size_t*)realloc(store->d_counts, sizeof(*store->d_counts) * dsize);

	if (!store->litlens || !store->dists) exit(-1);
	if (!store->pos) exit(-1);
	if (!store->ll_symbol || !store->d_symbol) exit(-1);
	if (!store->ll_counts || !store->d_counts) exit(-1);
}

static size_t CeilDiv(size_t a, size_t b) {
	return (a + b - 1) / b;
}

void ZopfliCopyLZ77Store(const ZopfliLZ77Store* source, ZopfliLZ77Store* dest) {
	size_t llsize = ZOPFLI_NUM_LL * CeilDiv(source->size, ZOPFLI_NUM_LL);
	size_t dsize = ZOPFLI_NUM_D * CeilDiv(source->size, ZOPFLI_NUM_D);
	ZopfliReInitLZ77Store(source->data, dest, source->size, llsize, dsize);

	memcpy(dest->litlens, source->litlens, sizeof(unsigned short) * source->size);
	memcpy(dest->dists, source->dists, sizeof(unsigned short) * source->size);
	memcpy(dest->pos, source->pos, sizeof(size_t) * source->size);
	memcpy(dest->ll_symbol, source->ll_symbol, sizeof(unsigned short) * source->size);
	memcpy(dest->d_symbol, source->d_symbol, sizeof(unsigned short) * source->size);

	memcpy(dest->ll_counts, source->ll_counts, sizeof(size_t) * llsize);
	memcpy(dest->d_counts, source->d_counts, sizeof(size_t) * dsize);
}

/*
Appends the length and distance to the LZ77 arrays of the ZopfliLZ77Store.
context must be a ZopfliLZ77Store*.
*/
void ZopfliStoreLitLenDist(
	unsigned short length, unsigned short dist, size_t pos, ZopfliLZ77Store* store) {
	size_t i;
	/* Needed for using ZOPFLI_APPEND_DATA multiple times. */
	size_t origsize = store->size;
	size_t llstart = ZOPFLI_NUM_LL * (origsize / ZOPFLI_NUM_LL);
	size_t dstart = ZOPFLI_NUM_D * (origsize / ZOPFLI_NUM_D);

	/* Everytime the index wraps around, a new cumulative histogram is made: we're
	keeping one histogram value per LZ77 symbol rather than a full histogram for
	each to save memory. */
	if (origsize % ZOPFLI_NUM_LL == 0) {
		size_t llsize = origsize;
		for (i = 0; i < ZOPFLI_NUM_LL; i++) {
			ZOPFLI_APPEND_DATA(
				origsize == 0 ? 0 : store->ll_counts[origsize - ZOPFLI_NUM_LL + i],
				&store->ll_counts, &llsize
			);
		}
	}
	if (origsize % ZOPFLI_NUM_D == 0) {
		size_t dsize = origsize;
		for (i = 0; i < ZOPFLI_NUM_D; i++) {
			ZOPFLI_APPEND_DATA(
				origsize == 0 ? 0 : store->d_counts[origsize - ZOPFLI_NUM_D + i],
				&store->d_counts, &dsize
			);
		}
	}

	ZOPFLI_APPEND_DATA(length, &store->litlens, &store->size);
	store->size = origsize;
	ZOPFLI_APPEND_DATA(dist, &store->dists, &store->size);
	store->size = origsize;
	ZOPFLI_APPEND_DATA(pos, &store->pos, &store->size);
	assert(length < 259);

	if (dist == 0) {
		store->size = origsize;
		ZOPFLI_APPEND_DATA(length, &store->ll_symbol, &store->size);
		store->size = origsize;
		ZOPFLI_APPEND_DATA(0, &store->d_symbol, &store->size);
		store->ll_counts[llstart + length]++;
	} else {
		store->size = origsize;
		ZOPFLI_APPEND_DATA(ZopfliGetLengthSymbol(length), &store->ll_symbol, &store->size);
		store->size = origsize;
		ZOPFLI_APPEND_DATA(ZopfliGetDistSymbol(dist), &store->d_symbol, &store->size);
		store->ll_counts[llstart + ZopfliGetLengthSymbol(length)]++;
		store->d_counts[dstart + ZopfliGetDistSymbol(dist)]++;
	}
}

void ZopfliAppendLZ77Store(const ZopfliLZ77Store* store, ZopfliLZ77Store* target) {
	size_t i;
	for (i = 0; i < store->size; i++) {
		ZopfliStoreLitLenDist(store->litlens[i], store->dists[i], store->pos[i], target);
	}
}
