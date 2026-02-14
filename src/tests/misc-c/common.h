/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _COMMON_H_
#define _COMMON_H_

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>

//==================================================================================================
// Macros
//==================================================================================================

/**
 * @brief Performs a static assertion.
 *
 * @param a Expression to assert.
 * @param b Expected value.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT(a, b) ((void)sizeof(char[(((a) == (b)) ? 1 : -1)]))

/**
 * @brief Performs a static assertion on the size of a type.
 *
 * @param a Type to assert.
 * @param b Expected size.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_SIZE(a, b) STATIC_ASSERT(sizeof(a), b)

/**
 * @brief Performs a static assertion on the alignment of a type.
 *
 * @param a Type to assert.
 * @param b Expected alignment.
 *
 * @returns Nothing. If the assertion fails, compilation will fail.
 */
#define STATIC_ASSERT_ALIGNMENT(a, b) STATIC_ASSERT(_Alignof(a), b)

//==================================================================================================
// Functions
//==================================================================================================

// Tests whether we can get the resolution of a clock with `clock_getres()`.
extern void test_clock_getres(void);

// Tests if `getenv()` works.
extern void test_getenv(void);

// Tests whether we can get the effective group ID of the calling process with `getegid()`.
extern void test_getegid(void);

// Tests whether we can get the effective user ID of the calling process with `geteuid()`.
extern void test_geteuid(void);

// Tests whether we can get the group ID of the calling process with `getgid()`.
extern void test_getgid(void);

// Tests whether `gethostname()` works.
extern void test_gethostname(void);

// Tests whether we can the user ID of the calling process with `getuid()`.
extern void test_getuid(void);

// Tests whether we can get the current time of a clock with `clock_gettime()`.
extern void test_clock_gettime(void);

// Tests whether we can sleep for a given amount of time with `nanosleep()`.
extern void test_nanosleep(void);

// Tests whether `setegid()` can be used to set the effective group ID of the calling process.
extern void test_setegid(void);

// Tests whether `setgid()` can be used to set the real group ID of the calling process.
extern void test_setgid(void);

// Tests whether `seteuid()` can be used to set the effective user ID of the calling process.
extern void test_seteuid(void);

// Tests whether `setuid()` can be used to set the real user ID of the calling process.
extern void test_setuid(void);

// Tests whether we can retrieve process times with `times()`.
extern void test_times(void);

// Tests whether we can get system information with `uname()`.
extern void test_uname(void);

#endif
