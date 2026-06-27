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

// Pattern byte used to fill memory before realloc.
#define FILL_BYTE 0xA5u

// Number of grow/shrink cycles in the stress test.
#define STRESS_CYCLES 64u

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests realloc growth, shrink, NULL-ptr, data preservation, alignment boundary sizes, and
// repeated grow/shrink cycles. Exercises the BlockHeader alignment rounding and free-path
// header validation introduced by the libc_stdlib fix.
void test_realloc(void)
{
    fprintf(stderr, "testing realloc() ... ");

    //==============================================================================================
    // Growth path: malloc then realloc to a larger size.
    //==============================================================================================

    {
        void *ptr = malloc(64u);
        assert(ptr != NULL);
        memset(ptr, FILL_BYTE, 64u);

        void *ptr2 = realloc(ptr, 256u);
        assert(ptr2 != NULL);

        // Verify original data is preserved in the first 64 bytes.
        unsigned char *bytes = (unsigned char *)ptr2;
        for (size_t i = 0; i < 64u; ++i) {
            assert(bytes[i] == FILL_BYTE);
        }

        free(ptr2);
    }

    //==============================================================================================
    // Shrink path: malloc a large block then realloc to a smaller size.
    //==============================================================================================

    {
        void *ptr = malloc(512u);
        assert(ptr != NULL);
        memset(ptr, FILL_BYTE, 512u);

        void *ptr2 = realloc(ptr, 64u);
        assert(ptr2 != NULL);

        // Verify data preserved in the smaller region.
        unsigned char *bytes = (unsigned char *)ptr2;
        for (size_t i = 0; i < 64u; ++i) {
            assert(bytes[i] == FILL_BYTE);
        }

        free(ptr2);
    }

    //==============================================================================================
    // NULL-ptr realloc: behaves like malloc.
    //==============================================================================================

    {
        void *ptr = realloc(NULL, 128u);
        assert(ptr != NULL);
        memset(ptr, FILL_BYTE, 128u);

        unsigned char *bytes = (unsigned char *)ptr;
        for (size_t i = 0; i < 128u; ++i) {
            assert(bytes[i] == FILL_BYTE);
        }

        free(ptr);
    }

    //==============================================================================================
    // Alignment boundary sizes: exercise UNDERLYING_ALIGNMENT (8-byte) rounding.
    //==============================================================================================

    {
        const size_t sizes[] = {1u, 7u, 8u, 9u, 15u, 16u, 17u, 31u, 32u, 33u};
        const size_t nsizes = sizeof(sizes) / sizeof(sizes[0]);

        for (size_t i = 0; i < nsizes; ++i) {
            size_t sz = sizes[i];
            void *ptr = malloc(sz);
            assert(ptr != NULL);
            memset(ptr, FILL_BYTE, sz);

            // Grow to double size.
            size_t new_sz = sz * 2;
            void *ptr2 = realloc(ptr, new_sz);
            assert(ptr2 != NULL);

            // Verify original data preserved.
            unsigned char *bytes = (unsigned char *)ptr2;
            for (size_t j = 0; j < sz; ++j) {
                assert(bytes[j] == FILL_BYTE);
            }

            free(ptr2);
        }
    }

    //==============================================================================================
    // Repeated grow/shrink cycles: stress alignment and header validation.
    //==============================================================================================

    {
        size_t cur_size = 32u;
        void *ptr = malloc(cur_size);
        assert(ptr != NULL);
        memset(ptr, FILL_BYTE, cur_size);

        for (size_t cycle = 0; cycle < STRESS_CYCLES; ++cycle) {
            // Alternate: grow on even cycles, shrink on odd.
            size_t new_size = (cycle % 2 == 0) ? cur_size * 2 : cur_size / 2;
            if (new_size == 0) {
                new_size = 1;
            }

            size_t check_size = (new_size < cur_size) ? new_size : cur_size;
            void *ptr2 = realloc(ptr, new_size);
            assert(ptr2 != NULL);

            // Verify data preserved up to the smaller of old/new sizes.
            unsigned char *bytes = (unsigned char *)ptr2;
            for (size_t j = 0; j < check_size; ++j) {
                assert(bytes[j] == FILL_BYTE);
            }

            // Fill the new region with the pattern.
            memset(ptr2, FILL_BYTE, new_size);

            ptr = ptr2;
            cur_size = new_size;
        }

        free(ptr);
    }

    fprintf(stderr, "passed\n");
}
