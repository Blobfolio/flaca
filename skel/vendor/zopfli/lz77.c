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

#include "lz77.h"
#include "symbols.h"
#include "util.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

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

size_t ZopfliLZ77GetByteRange(const ZopfliLZ77Store* lz77, size_t lstart, size_t lend) {
	size_t l = lend - 1;
	if (lstart == lend) return 0;
	return lz77->pos[l] + ((lz77->dists[l] == 0) ? 1 : lz77->litlens[l]) - lz77->pos[lstart];
}

static void ZopfliLZ77GetHistogramAt(
	const ZopfliLZ77Store* lz77, size_t lpos, size_t* ll_counts, size_t* d_counts) {
	/* The real histogram is created by using the histogram for this chunk, but
	all superfluous values of this chunk subtracted. */
	size_t llpos = ZOPFLI_NUM_LL * (lpos / ZOPFLI_NUM_LL);
	size_t dpos = ZOPFLI_NUM_D * (lpos / ZOPFLI_NUM_D);
	size_t i;
	for (i = 0; i < ZOPFLI_NUM_LL; i++) {
		ll_counts[i] = lz77->ll_counts[llpos + i];
	}
	for (i = lpos + 1; i < llpos + ZOPFLI_NUM_LL && i < lz77->size; i++) {
		ll_counts[lz77->ll_symbol[i]]--;
	}
	for (i = 0; i < ZOPFLI_NUM_D; i++) {
		d_counts[i] = lz77->d_counts[dpos + i];
	}
	for (i = lpos + 1; i < dpos + ZOPFLI_NUM_D && i < lz77->size; i++) {
		if (lz77->dists[i] != 0) d_counts[lz77->d_symbol[i]]--;
	}
}

void ZopfliLZ77GetHistogram(
	const ZopfliLZ77Store* lz77, size_t lstart, size_t lend,
	size_t* ll_counts, size_t* d_counts) {
	size_t i;
	if (lstart + ZOPFLI_NUM_LL * 3 > lend) {
		memset(ll_counts, 0, sizeof(*ll_counts) * ZOPFLI_NUM_LL);
		memset(d_counts, 0, sizeof(*d_counts) * ZOPFLI_NUM_D);
		for (i = lstart; i < lend; i++) {
			ll_counts[lz77->ll_symbol[i]]++;
			if (lz77->dists[i] != 0) d_counts[lz77->d_symbol[i]]++;
		}
	} else {
		/* Subtract the cumulative histograms at the end and the start to get the
		histogram for this range. */
		ZopfliLZ77GetHistogramAt(lz77, lend - 1, ll_counts, d_counts);
		if (lstart > 0) {
			size_t ll_counts2[ZOPFLI_NUM_LL];
			size_t d_counts2[ZOPFLI_NUM_D];
			ZopfliLZ77GetHistogramAt(lz77, lstart - 1, ll_counts2, d_counts2);

			for (i = 0; i < ZOPFLI_NUM_LL; i++) {
				ll_counts[i] -= ll_counts2[i];
			}
			for (i = 0; i < ZOPFLI_NUM_D; i++) {
				d_counts[i] -= d_counts2[i];
			}
		}
	}
}
