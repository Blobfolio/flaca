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

#include "squeeze.h"

#include <assert.h>
#include <math.h>
#include <stdio.h>

#include "../rust.h"
#include "blocksplitter.h"
#include "deflate.h"
#include "symbols.h"
#include "util.h"

/* Sets everything to 0. */
static void InitStats(SymbolStats* stats) {
	memset(stats->litlens, 0, ZOPFLI_NUM_LL * sizeof(size_t));
	memset(stats->dists, 0, ZOPFLI_NUM_D * sizeof(size_t));

	memset(stats->ll_symbols, 0, ZOPFLI_NUM_LL * sizeof(double));
	memset(stats->d_symbols, 0, ZOPFLI_NUM_D * sizeof(double));
}

static void CopyStats(SymbolStats* source, SymbolStats* dest) {
	memcpy(dest->litlens, source->litlens, ZOPFLI_NUM_LL * sizeof(size_t));
	memcpy(dest->dists, source->dists, ZOPFLI_NUM_D * sizeof(size_t));

	memcpy(dest->ll_symbols, source->ll_symbols, ZOPFLI_NUM_LL * sizeof(double));
	memcpy(dest->d_symbols, source->d_symbols, ZOPFLI_NUM_D * sizeof(double));
}

/* Adds the bit lengths. */
static void AddWeighedStatFreqs(
	const SymbolStats* stats1, double w1,
	const SymbolStats* stats2, double w2,
	SymbolStats* result) {
	size_t i;
	for (i = 0; i < ZOPFLI_NUM_LL; i++) {
		result->litlens[i] = (size_t) (stats1->litlens[i] * w1 + stats2->litlens[i] * w2);
	}
	for (i = 0; i < ZOPFLI_NUM_D; i++) {
		result->dists[i] = (size_t) (stats1->dists[i] * w1 + stats2->dists[i] * w2);
	}
	result->litlens[256] = 1;  /* End symbol. */
}

typedef struct RanState {
	unsigned int m_w, m_z;
} RanState;

static void InitRanState(RanState* state) {
	state->m_w = 1;
	state->m_z = 2;
}

/* Get random number: "Multiply-With-Carry" generator of G. Marsaglia */
static unsigned int Ran(RanState* state) {
	state->m_z = 36969 * (state->m_z & 65535) + (state->m_z >> 16);
	state->m_w = 18000 * (state->m_w & 65535) + (state->m_w >> 16);
	return (state->m_z << 16) + state->m_w;  /* 32-bit result. */
}

static void RandomizeFreqs(RanState* state, size_t* freqs, int n) {
	int i;
	for (i = 0; i < n; i++) {
		if ((Ran(state) >> 4) % 3 == 0) freqs[i] = freqs[Ran(state) % n];
	}
}

static void RandomizeStatFreqs(RanState* state, SymbolStats* stats) {
	RandomizeFreqs(state, stats->litlens, ZOPFLI_NUM_LL);
	RandomizeFreqs(state, stats->dists, ZOPFLI_NUM_D);
	stats->litlens[256] = 1;  /* End symbol. */
}

static void ClearStatFreqs(SymbolStats* stats) {
	memset(stats->litlens, 0, ZOPFLI_NUM_LL * sizeof(size_t));
	memset(stats->dists, 0, ZOPFLI_NUM_D * sizeof(size_t));
}

/* Calculates the entropy of the statistics */
static void CalculateStatistics(SymbolStats* stats) {
	ZopfliCalculateEntropy(stats->litlens, ZOPFLI_NUM_LL, stats->ll_symbols);
	ZopfliCalculateEntropy(stats->dists, ZOPFLI_NUM_D, stats->d_symbols);
}

/* Appends the symbol statistics from the store. */
static void GetStatistics(const ZopfliLZ77Store* store, SymbolStats* stats) {
	size_t i;
	for (i = 0; i < store->size; i++) {
		if (store->dists[i] == 0) {
			stats->litlens[store->litlens[i]]++;
		} else {
			stats->litlens[ZopfliGetLengthSymbol(store->litlens[i])]++;
			stats->dists[ZopfliGetDistSymbol(store->dists[i])]++;
		}
	}
	stats->litlens[256] = 1;  /* End symbol. */

	CalculateStatistics(stats);
}


void ZopfliLZ77Optimal(
	const unsigned char* in, size_t instart, size_t inend,
	int numiterations, ZopfliLZ77Store* store) {
	/* Dist to get to here with smallest cost. */
	size_t blocksize = inend - instart;
	ZopfliLZ77Store currentstore;
	SymbolStats stats, beststats, laststats;
	int i;
	double cost;
	double bestcost = ZOPFLI_LARGE_FLOAT;
	double lastcost = 0;
	/* Try randomizing the costs a bit once the size stabilizes. */
	RanState ran_state;
	int lastrandomstep = -1;

	InitRanState(&ran_state);
	InitStats(&stats);
	ZopfliInitLZ77Store(in, &currentstore);
	ZopfliInitCache(blocksize);

	/* Do regular deflate, then loop multiple shortest path runs, each time using
	the statistics of the previous run. */

	/* Initial run. */
	ZopfliLZ77Greedy(1, in, instart, inend, &currentstore);
	GetStatistics(&currentstore, &stats);

	/* Repeat statistics with each time the cost model from the previous stat
	run. */
	for (i = 0; i < numiterations; i++) {
		ZopfliCleanLZ77Store(&currentstore);
		ZopfliInitLZ77Store(in, &currentstore);
		LZ77OptimalRun(in, instart, inend, &stats, &currentstore);
		cost = ZopfliCalculateBlockSize(&currentstore, 0, currentstore.size, 2);
		if (cost < bestcost) {
			/* Copy to the output store. */
			ZopfliCopyLZ77Store(&currentstore, store);
			CopyStats(&stats, &beststats);
			bestcost = cost;
		}
		CopyStats(&stats, &laststats);
		ClearStatFreqs(&stats);
		GetStatistics(&currentstore, &stats);
		if (lastrandomstep != -1) {
			/* This makes it converge slower but better. Do it only once the
			randomness kicks in so that if the user does few iterations, it gives a
			better result sooner. */
			AddWeighedStatFreqs(&stats, 1.0, &laststats, 0.5, &stats);
			CalculateStatistics(&stats);
		}
		if (i > 5 && cost == lastcost) {
			CopyStats(&beststats, &stats);
			RandomizeStatFreqs(&ran_state, &stats);
			CalculateStatistics(&stats);
			lastrandomstep = i;
		}
		lastcost = cost;
	}

	ZopfliCleanLZ77Store(&currentstore);
}

void ZopfliLZ77OptimalFixed(
	const unsigned char* in, size_t instart, size_t inend, ZopfliLZ77Store* store) {
	/* Dist to get to here with smallest cost. */
	size_t blocksize = inend - instart;

	/* Shortest path for fixed tree This one should give the shortest possible
	result for fixed tree, no repeated runs are needed since the tree is known. */
	ZopfliInitCache(blocksize);
	LZ77OptimalRun(in, instart, inend, 0, store);
}
