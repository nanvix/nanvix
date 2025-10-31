/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <stdio.h>
#include <sys/mman.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can map and unmap memory using `mmap()` and `munmap()`.
void test_mmap_munmap(void)
{
    fprintf(stderr, "testing mmap() and munmap() with anonymous memory ... ");

    // Get the page size from the system.
    long page_size = sysconf(_SC_PAGE_SIZE);
    assert(page_size > 0);

    // Map a page of anonymous memory.
    void *ptr = mmap(NULL, page_size, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    assert(ptr != MAP_FAILED);

    // Attempt to write to the mapped memory.
    *((char *)ptr) = 'A';

    // Unmap the page.
    assert(munmap(ptr, page_size) == 0);

    fprintf(stderr, "passed\n");
}
