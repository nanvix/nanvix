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
#include <stdio.h>
#include <sys/stat.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether `mkfifo()` reports that FIFO special files are unsupported.
void test_mkfifo(void)
{
    fprintf(stderr, "testing mkfifo() ... ");

    // Nanvix's filesystem does not support FIFO special files, so the call must fail with ENOTSUP
    // rather than misleadingly claiming the function itself is unimplemented.
    errno = 0;
    assert(mkfifo("fifo", S_IRUSR | S_IWUSR) == -1);
    assert(errno == ENOTSUP);

    // Restore a clean errno so subsequent tests are not affected by this error path.
    errno = 0;

    fprintf(stderr, "passed\n");
}
