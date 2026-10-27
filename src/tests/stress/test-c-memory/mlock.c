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
// Private Functions
//==================================================================================================

// Maps anonymous memory that can be read and written, and fills it with a known value.
static unsigned char *map_filled(size_t length, unsigned char value)
{
    unsigned char *ptr =
        mmap(NULL, length, PROT_READ | PROT_WRITE, MAP_PRIVATE | MAP_ANONYMOUS, -1, 0);
    assert(ptr != MAP_FAILED);
    memset(ptr, value, length);
    return ptr;
}

// Asserts that every byte of a range of memory holds a known value.
static void assert_filled(const unsigned char *ptr, size_t length, unsigned char value)
{
    for (size_t i = 0; i < length; ++i) {
        assert(ptr[i] == value);
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can lock and unlock ranges of mapped memory with `mlock()` and `munlock()`.
void test_mlock_munlock(void)
{
    fprintf(stderr, "testing mlock() and munlock() with anonymous memory ... ");

    // Get the page size from the system.
    long page_size = sysconf(_SC_PAGE_SIZE);
    assert(page_size > 0);

    // Map two pages of anonymous memory and fill them with a known pattern.
    const size_t length = 2u * (size_t)page_size;
    unsigned char *ptr = map_filled(length, 0xA5u);

    // Locked pages must keep their contents and must remain readable and writable, and unlocking
    // them must preserve their contents too.
    {
        assert(mlock(ptr, length) == 0);
        assert_filled(ptr, length, 0xA5u);
        memset(ptr, 0x5A, length);
        assert_filled(ptr, length, 0x5Au);

        assert(munlock(ptr, length) == 0);
        assert_filled(ptr, length, 0x5Au);
    }

    // A lock may cover part of the mapping, and a partial page stands for the whole page.
    {
        assert(mlock(ptr + page_size, (size_t)page_size) == 0);
        assert(munlock(ptr + page_size, (size_t)page_size) == 0);

        assert(mlock(ptr, 1u) == 0);
        assert(munlock(ptr, 1u) == 0);
    }

    // Locks do not stack, so locking pages that are already locked and unlocking pages that are
    // not locked both succeed.
    {
        assert(mlock(ptr, length) == 0);
        assert(mlock(ptr, length) == 0);
        assert(munlock(ptr, length) == 0);
        assert(munlock(ptr, length) == 0);
    }

    // A zero-length range is accepted.
    assert(mlock(ptr, 0u) == 0);
    assert(munlock(ptr, 0u) == 0);

    // A base address that is not page-aligned is rejected, as POSIX permits.
    {
        errno = 0;
        assert(mlock(ptr + 1, (size_t)page_size) == -1);
        assert(errno == EINVAL);

        errno = 0;
        assert(munlock(ptr + 1, (size_t)page_size) == -1);
        assert(errno == EINVAL);
    }

    // A range that is not mapped is rejected. The page at the null address is never mapped.
    {
        errno = 0;
        assert(mlock(NULL, (size_t)page_size) == -1);
        assert(errno == ENOMEM);

        errno = 0;
        assert(munlock(NULL, (size_t)page_size) == -1);
        assert(errno == ENOMEM);
    }

    // Unmap the pages.
    assert(munmap(ptr, length) == 0);

    fprintf(stderr, "passed\n");
}

// Tests whether we can lock and unlock the whole address space with `mlockall()` and
// `munlockall()`.
void test_mlockall_munlockall(void)
{
    fprintf(stderr, "testing mlockall() and munlockall() with anonymous memory ... ");

    // Get the page size from the system.
    long page_size = sysconf(_SC_PAGE_SIZE);
    assert(page_size > 0);
    const size_t length = (size_t)page_size;

    // Map a page of anonymous memory and fill it with a known pattern.
    unsigned char *current = map_filled(length, 0xA5u);

    // Locking the pages that are currently mapped must preserve their contents.
    assert(mlockall(MCL_CURRENT) == 0);
    assert_filled(current, length, 0xA5u);

    // Pages that become mapped while future locking is in effect must be usable.
    assert(mlockall(MCL_CURRENT | MCL_FUTURE) == 0);
    unsigned char *future = map_filled(length, 0x5Au);
    assert_filled(future, length, 0x5Au);
    assert_filled(current, length, 0xA5u);

    // Unlocking the whole address space must preserve the contents of the pages.
    assert(munlockall() == 0);
    assert_filled(current, length, 0xA5u);
    assert_filled(future, length, 0x5Au);

    // Future locking may be requested on its own, and unlocking the address space when no page is
    // locked is not an error.
    assert(mlockall(MCL_FUTURE) == 0);
    assert(munlockall() == 0);
    assert(munlockall() == 0);

    // Flags must request at least one kind of locking.
    {
        errno = 0;
        assert(mlockall(0) == -1);
        assert(errno == EINVAL);
    }

    // Unknown flags are rejected, alone or combined with known ones.
    {
        const int unknown = ~(MCL_CURRENT | MCL_FUTURE);

        errno = 0;
        assert(mlockall(unknown) == -1);
        assert(errno == EINVAL);

        errno = 0;
        assert(mlockall(MCL_CURRENT | unknown) == -1);
        assert(errno == EINVAL);
    }

    // Unmap the pages.
    assert(munmap(future, length) == 0);
    assert(munmap(current, length) == 0);

    fprintf(stderr, "passed\n");
}
