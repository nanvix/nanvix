/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//==================================================================================================
// Constants
//==================================================================================================

// 1 KB.
#define KB (1024u)

// 1 MB.
#define MB (1024u * KB)

// Number of alloc/free rounds for the reclaim test.
#define RECLAIM_ROUNDS 32u

// Number of allocations per round.
#define ALLOCS_PER_ROUND 8u

// Block sizes to cycle through.
#define NUM_BLOCK_SIZES 4u
static const size_t BLOCK_SIZES[NUM_BLOCK_SIZES] = {
    512u * KB,
    256u * KB,
    128u * KB,
    64u * KB,
};

// Size of small anchor allocations placed between freed blocks.
#define ANCHOR_SIZE 64u

// xorshift PRNG seed.
#define XORSHIFT_SEED 0xDEADBEEFu

// Size of each large block used to fill the heap.
#define FILL_BLOCK_SIZE MB

// Maximum number of large blocks to attempt (32 MB / 1 MB).
#define MAX_FILL_BLOCKS 32u

// Size of medium blocks used in the fragmentation phase.
#define MEDIUM_BLOCK_SIZE (256u * KB)

// Maximum number of medium blocks (32 MB / 256 KB).
#define MAX_MEDIUM_BLOCKS 128u

// Number of allocations in the reuse-after-drain validation.
#define REUSE_ROUNDS 8u

// Size of blocks allocated in the reuse-after-drain phase.
#define REUSE_BLOCK_SIZE (512u * KB)

//==================================================================================================
// Private Functions
//==================================================================================================

// Simple 32-bit xorshift PRNG.
static unsigned xorshift32(unsigned state)
{
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;
    return state;
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Stresses the heap allocator with many alloc/free cycles of varying sizes under fragmentation
// pressure. Each round allocates blocks of pseudo-random sizes, interleaves them with small
// persistent anchors, and frees the large blocks while keeping the anchors.
void test_heap_reclaim(void)
{
    fprintf(stderr, "testing heap reclaim ... ");

    // Anchors survive across rounds to create fragmentation.
    unsigned char *anchors[RECLAIM_ROUNDS * ALLOCS_PER_ROUND];
    size_t num_anchors = 0;
    unsigned rng = XORSHIFT_SEED;

    for (size_t round = 0; round < RECLAIM_ROUNDS; ++round) {
        unsigned char *blocks[ALLOCS_PER_ROUND];

        for (size_t slot = 0; slot < ALLOCS_PER_ROUND; ++slot) {
            // Pick a block size pseudo-randomly.
            rng = xorshift32(rng);
            size_t size_index = rng % NUM_BLOCK_SIZES;
            size_t sz = BLOCK_SIZES[size_index];

            blocks[slot] = (unsigned char *)malloc(sz);
            assert(blocks[slot] != NULL);

            // Tag the first byte so the allocation is not optimized away.
            unsigned char tag = (unsigned char)((round * ALLOCS_PER_ROUND + slot) & 0xFFu);
            blocks[slot][0] = tag;
            assert(blocks[slot][0] == tag);

            // Place an anchor after every second block.
            if (slot % 2 == 1) {
                anchors[num_anchors] = (unsigned char *)malloc(ANCHOR_SIZE);
                assert(anchors[num_anchors] != NULL);
                anchors[num_anchors][0] = (unsigned char)((round + slot) & 0xFFu);
                num_anchors++;
            }
        }

        // Free all large blocks from this round; anchors persist.
        for (size_t slot = 0; slot < ALLOCS_PER_ROUND; ++slot) {
            free(blocks[slot]);
        }
    }

    // Clean up all anchors.
    for (size_t i = 0; i < num_anchors; ++i) {
        free(anchors[i]);
    }

    fprintf(stderr, "passed\n");
}

// Tests the allocator by attempting to consume the maximum allowed heap capacity. Proceeds in
// four phases: fill, drain, fragmentation, and reuse-after-drain.
void test_heap_max_capacity(void)
{
    fprintf(stderr, "testing heap max capacity ... ");

    //==============================================================================================
    // Phase 1: Fill the heap to near-capacity with large blocks.
    //==============================================================================================

    unsigned char *large_blocks[MAX_FILL_BLOCKS];
    size_t num_large = 0;
    size_t total_allocated = 0;

    for (size_t i = 0; i < MAX_FILL_BLOCKS; ++i) {
        unsigned char *p = (unsigned char *)malloc(FILL_BLOCK_SIZE);
        if (p == NULL) {
            break; // OOM — heap is full.
        }
        unsigned char tag = (unsigned char)(i & 0xFFu);
        memset(p, 0, FILL_BLOCK_SIZE);
        p[0] = tag;
        large_blocks[num_large] = p;
        num_large++;
        total_allocated += FILL_BLOCK_SIZE;
    }

    // We should have allocated at least one block.
    assert(total_allocated >= FILL_BLOCK_SIZE);

    // Verify data integrity on all blocks.
    for (size_t i = 0; i < num_large; ++i) {
        unsigned char expected = (unsigned char)(i & 0xFFu);
        assert(large_blocks[i][0] == expected);
    }

    //==============================================================================================
    // Phase 2: Drain and verify reclaimability.
    //==============================================================================================

    for (size_t i = 0; i < num_large; ++i) {
        free(large_blocks[i]);
    }

    // Verify reclaimability with a multi-block allocation.
    size_t reclaim_size = (num_large >= 2) ? (2 * FILL_BLOCK_SIZE) : FILL_BLOCK_SIZE;
    unsigned char *reclaim = (unsigned char *)malloc(reclaim_size);
    assert(reclaim != NULL);
    memset(reclaim, 0xAA, reclaim_size);
    free(reclaim);

    //==============================================================================================
    // Phase 3: Fragmentation near capacity.
    //==============================================================================================

    unsigned char *medium_blocks[MAX_MEDIUM_BLOCKS];
    size_t num_medium = 0;

    for (size_t i = 0; i < MAX_MEDIUM_BLOCKS; ++i) {
        unsigned char *p = (unsigned char *)malloc(MEDIUM_BLOCK_SIZE);
        if (p == NULL) {
            break; // OOM — heap is full.
        }
        unsigned char tag = (unsigned char)(i & 0xFFu);
        memset(p, 0, MEDIUM_BLOCK_SIZE);
        p[0] = tag;
        medium_blocks[num_medium] = p;
        num_medium++;
    }

    // Retain odd-indexed blocks as anchors, free even-indexed ones.
    size_t anchor_count = 0;
    unsigned char *frag_anchors[MAX_MEDIUM_BLOCKS / 2];
    for (size_t i = 0; i < num_medium; ++i) {
        if (i % 2 == 1) {
            frag_anchors[anchor_count] = medium_blocks[i];
            anchor_count++;
        } else {
            free(medium_blocks[i]);
        }
    }

    // Allocate into the freed gaps.
    size_t num_gap = 0;
    unsigned char *gap_blocks[MAX_MEDIUM_BLOCKS / 2];
    for (size_t i = 0; i < anchor_count; ++i) {
        unsigned char *p = (unsigned char *)malloc(MEDIUM_BLOCK_SIZE);
        if (p == NULL) {
            break; // No more room.
        }
        unsigned char tag = (unsigned char)(i & 0xFFu);
        p[0] = tag;
        gap_blocks[num_gap] = p;
        num_gap++;
    }

    // We should have been able to fit at least some blocks into the gaps.
    if (anchor_count > 0) {
        assert(num_gap > 0);
    }

    // Verify gap block integrity.
    for (size_t i = 0; i < num_gap; ++i) {
        unsigned char expected = (unsigned char)(i & 0xFFu);
        assert(gap_blocks[i][0] == expected);
    }

    // Free all gap and anchor blocks.
    for (size_t i = 0; i < num_gap; ++i) {
        free(gap_blocks[i]);
    }
    for (size_t i = 0; i < anchor_count; ++i) {
        free(frag_anchors[i]);
    }

    //==============================================================================================
    // Phase 4: Reuse after full drain.
    //==============================================================================================

    for (size_t round = 0; round < REUSE_ROUNDS; ++round) {
        unsigned char *p = (unsigned char *)malloc(REUSE_BLOCK_SIZE);
        assert(p != NULL);
        unsigned char tag = (unsigned char)(round & 0xFFu);
        p[0] = tag;
        assert(p[0] == tag);
        free(p);
    }

    fprintf(stderr, "passed\n");
}
