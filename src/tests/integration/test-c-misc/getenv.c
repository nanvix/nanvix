/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests whether `getenv()` correctly retrieves a set environment variable.
 */
static void test_getenv_set(void)
{
    fprintf(stderr, "testing getenv() set ... ");

    const char *value = getenv("NANVIX_TEST");
    assert(value != NULL);
    assert(strcmp(value, "1") == 0);

    fprintf(stderr, "passed\n");
}

/**
 * @brief Tests whether `getenv()` returns NULL for an unset environment variable.
 */
static void test_getenv_unset(void)
{
    fprintf(stderr, "testing getenv() unset ... ");

    const char *value = getenv("NANVIX_UNSET_VAR");
    assert(value == NULL);

    fprintf(stderr, "passed\n");
}

/**
 * @brief Tests if `getenv()` works.
 */
void test_getenv(void)
{
    test_getenv_set();
    test_getenv_unset();
}
