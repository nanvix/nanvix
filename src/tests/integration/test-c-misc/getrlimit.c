/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <sys/resource.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Value that limits hold before each call. No resource has this limit, so limits that a call leaves
// unset are caught.
#define POISON ((rlim_t)0xdeadbeef)

//==================================================================================================
// Private Functions
//==================================================================================================

// Gets the limits on `resource` with `getrlimit()` and checks that they are consistent.
static struct rlimit get_limits(int resource)
{
    struct rlimit rl = {POISON, POISON};

    assert(getrlimit(resource, &rl) == 0);
    assert(rl.rlim_cur != POISON);
    assert(rl.rlim_max != POISON);

    // The soft limit cannot exceed the hard limit.
    assert(rl.rlim_cur <= rl.rlim_max);

    return (rl);
}

// Gets the soft limit on `resource` with `getrlimit()` and checks that it is finite and not zero.
static rlim_t get_finite_limit(int resource)
{
    const struct rlimit rl = get_limits(resource);

    assert(rl.rlim_cur != RLIM_INFINITY);
    assert(rl.rlim_cur > 0);

    return (rl.rlim_cur);
}

// Checks that `getrlimit()` reports that `resource` is not limited.
static void check_unlimited(int resource)
{
    const struct rlimit rl = get_limits(resource);

    assert(rl.rlim_cur == RLIM_INFINITY);
    assert(rl.rlim_max == RLIM_INFINITY);
}

// Checks that `getrlimit()` fails with `EINVAL` for `resource`, which is not a valid resource, and
// leaves the limits unchanged.
static void check_invalid_resource(int resource)
{
    struct rlimit rl = {POISON, POISON};

    errno = 0;
    assert(getrlimit(resource, &rl) == -1);
    assert(errno == EINVAL);
    assert(rl.rlim_cur == POISON);
    assert(rl.rlim_max == POISON);
}

// Checks that `malloc()` cannot allocate more memory than `limit`, the limit on the data segment.
static void check_data_limit(rlim_t limit)
{
    assert(limit < SIZE_MAX);

    errno = 0;
    assert(malloc((size_t)limit + 1) == NULL);
    assert(errno == ENOMEM);
}

// Checks that `limit`, the limit on file descriptors, is one greater than the largest descriptor
// that the system assigns.
static void check_nofile_limit(rlim_t limit)
{
    assert(limit <= (rlim_t)INT_MAX);

    // The largest descriptor below the limit can be assigned.
    const int highest = (int)limit - 1;
    assert(highest > STDERR_FILENO);
    assert(dup2(STDERR_FILENO, highest) == highest);
    assert(close(highest) == 0);

    // The descriptor at the limit cannot be assigned.
    errno = 0;
    assert(dup2(STDERR_FILENO, (int)limit) == -1);
    assert(errno == EBADF);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can get resource limits with `getrlimit()`.
void test_getrlimit(void)
{
    fprintf(stderr, "testing getrlimit() ... ");

    // Core images are never written.
    const struct rlimit core = get_limits(RLIMIT_CORE);
    assert(core.rlim_cur == 0);
    assert(core.rlim_max == 0);

    // CPU time, file sizes, resident set sizes, and locked memory are not limited.
    check_unlimited(RLIMIT_CPU);
    check_unlimited(RLIMIT_FSIZE);
    check_unlimited(RLIMIT_RSS);
    check_unlimited(RLIMIT_MEMLOCK);

    // The heap and the stack of the initial thread are limited, and they fit in the address space.
    const rlim_t data = get_finite_limit(RLIMIT_DATA);
    const rlim_t stack = get_finite_limit(RLIMIT_STACK);
    const rlim_t address_space = get_finite_limit(RLIMIT_AS);
    assert(data <= address_space);
    assert(stack <= address_space - data);
    check_data_limit(data);

    // The number of processes and the number of file descriptors are limited.
    (void)get_finite_limit(RLIMIT_NPROC);
    check_nofile_limit(get_finite_limit(RLIMIT_NOFILE));

    // Invalid resources and null pointers are rejected.
    check_invalid_resource(-1);
    check_invalid_resource(RLIMIT_NLIMITS);
    errno = 0;
    assert(getrlimit(RLIMIT_NOFILE, NULL) == -1);
    assert(errno == EINVAL);

    fprintf(stderr, "passed\n");
}
