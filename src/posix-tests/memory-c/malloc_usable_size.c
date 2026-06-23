/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <malloc.h>
#include <stddef.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can query the usable size of allocated blocks using `malloc_usable_size()`.
void test_malloc_usable_size(void)
{
    fprintf(stderr, "testing malloc_usable_size() ... ");

    // Null pointer must yield zero.
    {
        size_t sz = malloc_usable_size(NULL);
        assert(sz == 0u);
    }

    // Exercise a range of allocation sizes and verify reported usable size.
    {
        const size_t sizes[] = {1u, 8u, 32u, 64u, 127u, 128u, 255u, 256u, 511u, 512u, 1024u};
        const size_t nsizes = sizeof(sizes) / sizeof(sizes[0]);
        for (size_t i = 0; i < nsizes; ++i) {
            size_t req = sizes[i];
            void *p = malloc(req);
            assert(p != NULL);
            size_t usable = malloc_usable_size(p);
            // Current allocator promises exact size; enforce so regressions are caught.
            assert(usable >= req);
            assert(usable == req);
            // Touch within requested bounds (never rely on any extra capacity).
            memset(p, 0xA5, req);
            unsigned char *bytes = (unsigned char *)p;
            for (size_t j = 0; j < req; ++j) {
                assert(bytes[j] == 0xA5u);
            }
            free(p);
        }
    }

    // Realloc growth path: ensure updated usable size reflects new request.
    {
        void *p = malloc(64u);
        assert(p != NULL);
        assert(malloc_usable_size(p) == 64u);
        void *p2 = realloc(p, 200u);
        assert(p2 != NULL);
        assert(malloc_usable_size(p2) == 200u);
        memset(p2, 0x5A, 200u);
        free(p2);
    }

    // Aligned allocation path.
    {
        void *pa = aligned_alloc(128u, 256u); // size is a multiple of alignment.
        assert(pa != NULL);
        // Alignment validation (platform is 32-bit, so pointer fits in uintptr_t).
        assert(((uintptr_t)pa % 128u) == 0u);
        assert(malloc_usable_size(pa) == 256u);
        free(pa);
    }

    // Stress: allocate/free and query usable size repeatedly to reveal metadata issues.
    for (size_t iter = 0; iter < 100u; ++iter) {
        void *p = malloc(96u);
        assert(p != NULL);
        assert(malloc_usable_size(p) == 96u);
        ((unsigned char *)p)[0] = (unsigned char)iter; // Touch.
        free(p);
    }

    fprintf(stderr, "passed\n");
}
