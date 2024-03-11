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

#include "hash.h"

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>

#define HASH_SHIFT 5
#define HASH_MASK 32767
#define HASH_LENGTH 65536

void ZopfliAllocHash(ZopfliHash* h) {
  h->head = (int*)malloc(sizeof(*h->head) * HASH_LENGTH);
  h->prev = (unsigned short*)malloc(sizeof(*h->prev) * ZOPFLI_WINDOW_SIZE);
  h->hashval = (int*)malloc(sizeof(*h->hashval) * ZOPFLI_WINDOW_SIZE);

#ifdef ZOPFLI_HASH_SAME
  h->same = (unsigned short*)malloc(sizeof(*h->same) * ZOPFLI_WINDOW_SIZE);
#endif

#ifdef ZOPFLI_HASH_SAME_HASH
  h->head2 = (int*)malloc(sizeof(*h->head2) * HASH_LENGTH);
  h->prev2 = (unsigned short*)malloc(sizeof(*h->prev2) * ZOPFLI_WINDOW_SIZE);
  h->hashval2 = (int*)malloc(sizeof(*h->hashval2) * ZOPFLI_WINDOW_SIZE);
#endif
}

void ZopfliResetHash(ZopfliHash* h) {
  size_t i;

  h->val = 0;
  for (i = 0; i < HASH_LENGTH; i++) h->head[i] = -1;
  for (i = 0; i < ZOPFLI_WINDOW_SIZE; i++) h->prev[i] = i;

/* The default for hashval is -1 too. */
memcpy(h->hashval, h->head, sizeof(int) * ZOPFLI_WINDOW_SIZE);
#ifdef ZOPFLI_HASH_SAME
  /* 0 for no length. */
  memset(h->same, 0, sizeof(unsigned short) * ZOPFLI_WINDOW_SIZE);
#endif
#ifdef ZOPFLI_HASH_SAME_HASH
  h->val2 = 0;
  /* Copy what we already initialized. */
  memcpy(h->head2, h->head, sizeof(int) * HASH_LENGTH);
  memcpy(h->hashval2, h->hashval, sizeof(int) * ZOPFLI_WINDOW_SIZE);
  memcpy(h->prev2, h->prev, sizeof(unsigned short) * ZOPFLI_WINDOW_SIZE);
#endif

}

void ZopfliCleanHash(ZopfliHash* h) {
  free(h->head);
  free(h->prev);
  free(h->hashval);

#ifdef ZOPFLI_HASH_SAME_HASH
  free(h->head2);
  free(h->prev2);
  free(h->hashval2);
#endif

#ifdef ZOPFLI_HASH_SAME
  free(h->same);
#endif
}

/*
Update the sliding hash value with the given byte. All calls to this function
must be made on consecutive input characters. Since the hash value exists out
of multiple input bytes, a few warmups with this function are needed initially.
*/
static void UpdateHashValue(ZopfliHash* h, unsigned char c) {
  h->val = (((h->val) << HASH_SHIFT) ^ (c)) & HASH_MASK;
}

void ZopfliUpdateHash(const unsigned char* array, size_t pos, size_t end,
                ZopfliHash* h) {
  unsigned short hpos = pos & ZOPFLI_WINDOW_MASK;
#ifdef ZOPFLI_HASH_SAME
  size_t amount = 0;
#endif

  UpdateHashValue(h, pos + ZOPFLI_MIN_MATCH <= end ?
      array[pos + ZOPFLI_MIN_MATCH - 1] : 0);
  h->hashval[hpos] = h->val;
  if (h->head[h->val] != -1 && h->hashval[h->head[h->val]] == h->val) {
    h->prev[hpos] = h->head[h->val];
  }
  else h->prev[hpos] = hpos;
  h->head[h->val] = hpos;

#ifdef ZOPFLI_HASH_SAME
  /* Update "same". */
  if (h->same[(pos - 1) & ZOPFLI_WINDOW_MASK] > 1) {
    amount = h->same[(pos - 1) & ZOPFLI_WINDOW_MASK] - 1;
  }
  while (pos + amount + 1 < end &&
      array[pos] == array[pos + amount + 1] && amount < (unsigned short)(-1)) {
    amount++;
  }
  h->same[hpos] = amount;
#endif

#ifdef ZOPFLI_HASH_SAME_HASH
  h->val2 = ((h->same[hpos] - ZOPFLI_MIN_MATCH) & 255) ^ h->val;
  h->hashval2[hpos] = h->val2;
  if (h->head2[h->val2] != -1 && h->hashval2[h->head2[h->val2]] == h->val2) {
    h->prev2[hpos] = h->head2[h->val2];
  }
  else h->prev2[hpos] = hpos;
  h->head2[h->val2] = hpos;
#endif
}

void ZopfliWarmupHash(const unsigned char* array, size_t pos, size_t end,
                ZopfliHash* h) {
  UpdateHashValue(h, array[pos + 0]);
  if (pos + 1 < end) UpdateHashValue(h, array[pos + 1]);
}
