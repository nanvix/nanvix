/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <stdint.h> // uintptr_t
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * Tests whether we can allocate and free memory using `aligned_alloc()` and `free()`.
 * Validates that returned pointers satisfy the requested alignment, memory is writable,
 * and multiple allocation patterns work without failure.
 */
void test_aligned_alloc_free(void)
{
    fprintf(stderr, "testing aligned_alloc() and free() ... ");

    // A selection of alignment/size pairs. According to C11, alignment must be a power of two
    // and a multiple of sizeof(void*). Sizes must be a multiple of alignment.
    struct test_case {
        size_t alignment;
        size_t size; // Multiple of alignment.
    };

    const struct test_case cases[] = {
        {.alignment = 4u, .size = 4u}, // Assuming 32-bit (sizeof(void*) == 4).
        {.alignment = 8u, .size = 16u},
        {.alignment = 16u, .size = 32u},
        {.alignment = 32u, .size = 64u},
        {.alignment = 64u, .size = 128u},
        {.alignment = 128u, .size = 256u},
        {.alignment = 256u, .size = 512u},
        {.alignment = 512u, .size = 512u},
        {.alignment = 1024u, .size = 2048u},
    };

    const size_t ncases = sizeof(cases) / sizeof(cases[0]);

    for (size_t i = 0; i < ncases; ++i) {
        size_t alignment = cases[i].alignment;
        size_t size = cases[i].size;

        assert((alignment & (alignment - 1u)) == 0u); // Power of two.
        assert((alignment % sizeof(void *)) == 0u);   // Multiple of sizeof(void*).
        assert((size % alignment) == 0u);             // Required by aligned_alloc().

        void *ptr = aligned_alloc(alignment, size);
        assert(ptr != NULL);
        assert(((uintptr_t)ptr % alignment) == 0u);

        // Fill memory with a pattern and verify it was written.
        memset(ptr, (int)0x5Au, size);
        unsigned char *bytes = (unsigned char *)ptr;
        for (size_t j = 0; j < size; ++j) {
            assert(bytes[j] == 0x5Au);
        }

        free(ptr);
    }

    // Stress: allocate and free many aligned blocks of the same alignment.
    for (size_t iter = 0; iter < 64u; ++iter) {
        void *p = aligned_alloc(64u, 64u); // size multiple of alignment.
        assert(p != NULL);
        assert(((uintptr_t)p % 64u) == 0u);
        ((unsigned char *)p)[0] = (unsigned char)iter; // Touch.
        free(p);
    }

    fprintf(stderr, "passed\n");
}
