/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Configuration
//==================================================================================================

// Enable sbrk().
#define _BSD_SOURCE 1

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can allocate memory using `sbrk()`.
void test_sbrk(void)
{
    // Check that `sbrk()` returns a pointer to the end of the heap.
    void *ptr = sbrk(0);
    assert(ptr != (void *)-1);

    // Check that `sbrk()` can allocate memory.
    void *new_ptr = sbrk(4096);
    assert(new_ptr != (void *)-1);
    assert(ptr == new_ptr);

    // Check that program break is where we expect it to be.
    void *new_ptr2 = sbrk(0);
    assert(new_ptr2 != (void *)-1);
    assert(new_ptr2 == (void *)((char *)ptr + 4096));

    // Check that `sbrk()` can free memory.
    void *free_ptr = sbrk(-4096);
    assert(free_ptr != (void *)-1);
    assert(free_ptr == (void *)((char *)ptr + 4096));

    // Check that program break is where we expect it to be.
    void *new_ptr3 = sbrk(0);
    assert(new_ptr3 != (void *)-1);
    assert(new_ptr3 == ptr);
}
