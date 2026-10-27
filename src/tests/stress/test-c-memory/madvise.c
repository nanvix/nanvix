/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can advise the system about the use of mapped memory with `madvise()`.
void test_madvise(void)
{
    fprintf(stderr, "testing madvise() with anonymous memory ... ");

    // Get the page size from the system.
    long page_size = sysconf(_SC_PAGE_SIZE);
    assert(page_size > 0);

    // Map two pages of anonymous memory and fill them with a known pattern.
    const size_t length = 2u * (size_t)page_size;
    unsigned char *ptr =
        mmap(NULL, length, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    assert(ptr != MAP_FAILED);
    memset(ptr, 0xA5, length);

    // Advice about the access pattern or about upcoming accesses must be accepted and must preserve
    // the contents of the pages.
    {
        const int advice[] = {MADV_NORMAL, MADV_RANDOM, MADV_SEQUENTIAL, MADV_WILLNEED};
        const size_t nadvice = sizeof(advice) / sizeof(advice[0]);
        for (size_t i = 0; i < nadvice; ++i) {
            assert(madvise(ptr, length, advice[i]) == 0);
        }
        for (size_t i = 0; i < length; ++i) {
            assert(ptr[i] == 0xA5u);
        }
    }

    // Advice may cover part of the mapping, and a partial page stands for the whole page.
    {
        assert(madvise(ptr + page_size, (size_t)page_size, MADV_WILLNEED) == 0);
        assert(madvise(ptr, 1u, MADV_SEQUENTIAL) == 0);
    }

    // A zero-length range is accepted.
    assert(madvise(ptr, 0u, MADV_NORMAL) == 0);

    // Advice that the pages are no longer needed must be accepted and must leave them usable. Their
    // contents are not checked, because whether they are preserved differs across systems.
    {
        assert(madvise(ptr, length, MADV_DONTNEED) == 0);
        ptr[0] = 'A';
        assert(ptr[0] == 'A');

        assert(madvise(ptr, length, MADV_FREE) == 0);
        ptr[length - 1] = 'B';
        assert(ptr[length - 1] == 'B');
    }

    // Unknown advice is rejected.
    {
        errno = 0;
        assert(madvise(ptr, length, -1) == -1);
        assert(errno == EINVAL);
    }

    // A base address that is not page-aligned is rejected.
    {
        errno = 0;
        assert(madvise(ptr + 1, (size_t)page_size, MADV_NORMAL) == -1);
        assert(errno == EINVAL);
    }

    // A range that is not mapped is rejected. The page at the null address is never mapped.
    {
        errno = 0;
        assert(madvise(NULL, (size_t)page_size, MADV_NORMAL) == -1);
        assert(errno == ENOMEM);
    }

    // Unmap the pages.
    assert(munmap(ptr, length) == 0);

    fprintf(stderr, "passed\n");
}
