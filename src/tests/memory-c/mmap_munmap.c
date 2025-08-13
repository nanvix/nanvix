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

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Page size (in bytes.)
// TODO: get this from sysconf() when it is supported (#342).
#define PAGE_SIZE 4096

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can map and unmap memory using `mmap()` and `munmap()`.
void test_mmap_munmap(void)
{
    fprintf(stderr, "testing mmap() and munmap() with anonymous memory ... ");

    // Map a page of anonymous memory.
    void *ptr = mmap(NULL, PAGE_SIZE, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    assert(ptr != MAP_FAILED);

    // Attempt to write to the mapped memory.
    *((char *)ptr) = 'A';

    // Unmap the page.
    assert(munmap(ptr, PAGE_SIZE) == 0);

    fprintf(stderr, "passed\n");
}
