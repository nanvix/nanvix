/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Integration tests for the heap shrink path (Heap::shrink / try_reclaim).
 *
 * All scenarios exercise shrink() indirectly through malloc()/free(). The allocator triggers
 * try_reclaim() when the heap is at capacity (heap_size >= capacity), which calls
 * Heap::shrink() to unmap tail pages no longer containing live allocations.
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

// Block size used in bulk-free, anchor, and cycle scenarios.
#define LARGE_BLOCK_SIZE MB

// Number of large blocks in bulk-free and anchor scenarios.
#define NUM_LARGE_BLOCKS 4u

// Size of small anchor allocations.
#define ANCHOR_SIZE 64u

// Number of shrink-grow iterations.
#define SHRINK_GROW_CYCLES 16u

// Number of small allocations in the small-heap scenario.
#define NUM_SMALL_ALLOCS 8u

// Size of each small allocation (well below one page).
#define SMALL_ALLOC_SIZE 128u

// Number of regrowth blocks after shrink-to-minimum.
#define REGROWTH_BLOCKS 4u

// Size of each regrowth block (64 KB).
#define REGROWTH_BLOCK_SIZE (64u * KB)

//==================================================================================================
// Private Functions — Scenarios
//==================================================================================================

// Scenario 1: Bulk-free triggers shrink.
//
// Allocate several large blocks so the heap grows well beyond one page, then free them all.
// Each free at capacity triggers try_reclaim() → Heap::shrink() to unmap tail pages.
// Re-allocate the same total to confirm pages were reclaimed and can be re-mapped.
static void scenario_bulk_free(void)
{
    unsigned char *blocks[NUM_LARGE_BLOCKS];

    // Allocate large blocks.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        blocks[i] = (unsigned char *)malloc(LARGE_BLOCK_SIZE);
        assert(blocks[i] != NULL);
        memset(blocks[i], 0, LARGE_BLOCK_SIZE);
        blocks[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Verify integrity.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        assert(blocks[i][0] == (unsigned char)(i & 0xFFu));
    }

    // Free all — each free at capacity triggers try_reclaim() → shrink.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        free(blocks[i]);
    }

    // Re-allocate to confirm reclaimed pages are re-mappable.
    unsigned char *blocks2[NUM_LARGE_BLOCKS];
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        blocks2[i] = (unsigned char *)malloc(LARGE_BLOCK_SIZE);
        assert(blocks2[i] != NULL);
        memset(blocks2[i], 0, LARGE_BLOCK_SIZE);
        blocks2[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Verify integrity.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        assert(blocks2[i][0] == (unsigned char)(i & 0xFFu));
    }

    // Cleanup.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        free(blocks2[i]);
    }
}

// Scenario 2: All-freed shrinks to minimum.
//
// Allocate a single large block (2 MB) to force heap growth, free it. With no live allocations,
// try_reclaim() sees an empty span and shrinks the heap to PAGE_SIZE. Then allocate multiple
// medium blocks to verify the heap grows back from minimum.
static void scenario_shrink_to_minimum(void)
{
    // Grow the heap with a 2 MB allocation.
    unsigned char *block = (unsigned char *)malloc(2 * MB);
    assert(block != NULL);
    memset(block, 0, 2 * MB);
    block[0] = 0xAA;
    free(block);
    // Heap should now be at PAGE_SIZE (empty span → minimum).

    // Verify regrowth from minimum.
    unsigned char *regrowth[REGROWTH_BLOCKS];
    for (size_t i = 0; i < REGROWTH_BLOCKS; ++i) {
        regrowth[i] = (unsigned char *)malloc(REGROWTH_BLOCK_SIZE);
        assert(regrowth[i] != NULL);
        memset(regrowth[i], 0, REGROWTH_BLOCK_SIZE);
        regrowth[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Verify integrity.
    for (size_t i = 0; i < REGROWTH_BLOCKS; ++i) {
        assert(regrowth[i][0] == (unsigned char)(i & 0xFFu));
    }

    // Cleanup.
    for (size_t i = 0; i < REGROWTH_BLOCKS; ++i) {
        free(regrowth[i]);
    }
}

// Scenario 3: Anchors prevent full shrink.
//
// Allocate large blocks interleaved with small persistent anchors. Free the large blocks.
// try_reclaim() can only reclaim tail pages above the highest live anchor. Then free anchors
// and reallocate large blocks to verify full reclaimability.
static void scenario_anchors_prevent_full_shrink(void)
{
    unsigned char *blocks[NUM_LARGE_BLOCKS];
    unsigned char *anchors[NUM_LARGE_BLOCKS];

    // Interleave: large block, anchor, large block, anchor, ...
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        blocks[i] = (unsigned char *)malloc(LARGE_BLOCK_SIZE);
        assert(blocks[i] != NULL);
        memset(blocks[i], 0, LARGE_BLOCK_SIZE);
        blocks[i][0] = (unsigned char)(i & 0xFFu);

        anchors[i] = (unsigned char *)malloc(ANCHOR_SIZE);
        assert(anchors[i] != NULL);
        memset(anchors[i], 0, ANCHOR_SIZE);
        anchors[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Free large blocks; anchors persist and pin the heap above minimum.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        free(blocks[i]);
    }

    // Free anchors — now the heap can fully reclaim.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        free(anchors[i]);
    }

    // Verify full reclaimability by re-allocating large blocks.
    unsigned char *blocks2[NUM_LARGE_BLOCKS];
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        blocks2[i] = (unsigned char *)malloc(LARGE_BLOCK_SIZE);
        assert(blocks2[i] != NULL);
        memset(blocks2[i], 0, LARGE_BLOCK_SIZE);
        blocks2[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Verify integrity.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        assert(blocks2[i][0] == (unsigned char)(i & 0xFFu));
    }

    // Cleanup.
    for (size_t i = 0; i < NUM_LARGE_BLOCKS; ++i) {
        free(blocks2[i]);
    }
}

// Scenario 4: Repeated shrink-grow cycles.
//
// In a tight loop, allocate a large block then immediately free it. Each free triggers
// try_reclaim() → shrink(), and the next allocation triggers the OOM handler → grow().
// This exercises the self-healing path repeatedly.
static void scenario_shrink_grow_cycles(void)
{
    for (size_t i = 0; i < SHRINK_GROW_CYCLES; ++i) {
        unsigned char *block = (unsigned char *)malloc(LARGE_BLOCK_SIZE);
        assert(block != NULL);
        memset(block, 0, LARGE_BLOCK_SIZE);
        unsigned char tag = (unsigned char)(i & 0xFFu);
        block[0] = tag;
        assert(block[0] == tag);
        free(block);
    }
}

// Scenario 5: Small heap stays at minimum.
//
// Allocate only small blocks whose total is well below a single page. Free them. The heap
// should remain at PAGE_SIZE because try_reclaim() returns early when heap.size() <= PAGE_SIZE.
// Verify stability by allocating and freeing small blocks again.
static void scenario_small_heap_minimum(void)
{
    unsigned char *small[NUM_SMALL_ALLOCS];

    // Allocate small blocks that fit within the initial page.
    for (size_t i = 0; i < NUM_SMALL_ALLOCS; ++i) {
        small[i] = (unsigned char *)malloc(SMALL_ALLOC_SIZE);
        assert(small[i] != NULL);
        memset(small[i], 0, SMALL_ALLOC_SIZE);
        small[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Free all — heap remains at PAGE_SIZE (no shrink below minimum).
    for (size_t i = 0; i < NUM_SMALL_ALLOCS; ++i) {
        free(small[i]);
    }

    // Reallocate to verify stability.
    unsigned char *small2[NUM_SMALL_ALLOCS];
    for (size_t i = 0; i < NUM_SMALL_ALLOCS; ++i) {
        small2[i] = (unsigned char *)malloc(SMALL_ALLOC_SIZE);
        assert(small2[i] != NULL);
        memset(small2[i], 0, SMALL_ALLOC_SIZE);
        small2[i][0] = (unsigned char)(i & 0xFFu);
    }

    // Cleanup.
    for (size_t i = 0; i < NUM_SMALL_ALLOCS; ++i) {
        free(small2[i]);
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Exercises the heap shrink path through five scenarios that cover bulk-free, shrink-to-minimum,
// anchor-pinned partial shrink, repeated shrink-grow cycles, and small-heap early-return.
void test_heap_shrink(void)
{
    fprintf(stderr, "testing heap shrink ... ");

    scenario_bulk_free();
    scenario_shrink_to_minimum();
    scenario_anchors_prevent_full_shrink();
    scenario_shrink_grow_cycles();
    scenario_small_heap_minimum();

    fprintf(stderr, "passed\n");
}
