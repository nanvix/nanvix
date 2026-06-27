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
// Standalone Functions
//==================================================================================================

// Tests whether we can allocate and free memory using `malloc()` and `free()`.
void test_malloc_free(void)
{
    fprintf(stderr, "testing malloc() and free() ... ");

    // A small collection of allocation sizes (bytes) to exercise allocator paths.
    const size_t sizes[] = {1u, 8u, 32u, 128u, 511u, 512u, 1024u, 4096u};
    const size_t nsizes = sizeof(sizes) / sizeof(sizes[0]);

    // Allocate, touch, and free each block individually.
    for (size_t i = 0; i < nsizes; ++i) {
        size_t sz = sizes[i];
        void *ptr = malloc(sz);
        assert(ptr != NULL);

        // Fill memory with a pattern and verify it was written.
        memset(ptr, (int)(0xA5u), sz);
        unsigned char *bytes = (unsigned char *)ptr;
        for (size_t j = 0; j < sz; ++j) {
            assert(bytes[j] == 0xA5u);
        }

        free(ptr);
    }

    // Repeated allocate/free cycle to check for simple leaks or reuse issues.
    for (size_t iter = 0; iter < 100u; ++iter) {
        void *p = malloc(64u);
        assert(p != NULL);
        ((unsigned char *)p)[0] = (unsigned char)iter; // Touch.
        free(p);
    }

    fprintf(stderr, "passed\n");
}
