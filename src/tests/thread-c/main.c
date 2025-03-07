/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <unistd.h>

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
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests pthreads system calls.
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program will abort.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    // Sanity check size of `pthread_t` type.
    STATIC_ASSERT_SIZE(pthread_t, sizeof(uint32_t));

    // Sanity check size of `pthread_attr_t` type.
    STATIC_ASSERT_SIZE(pthread_attr_t,
                       sizeof(int) +                    // is_initialized
                           sizeof(void *) +             // stackaddr
                           sizeof(size_t) +             // stacksize
                           sizeof(int) +                // contentionscope
                           sizeof(int) +                // inheritsched
                           sizeof(int) +                // schedpolicy
                           sizeof(struct sched_param) + // schedparam
                           sizeof(int)                  // detachstate

    );

    // Sanity check size of `pthread_mutex_t` type.
    STATIC_ASSERT_SIZE(pthread_mutex_t, sizeof(uint32_t));

    // Sanity check size of `pthread_mutexattr_t` type.
    STATIC_ASSERT_SIZE(pthread_mutexattr_t,
                       sizeof(int) +     // is_initialized
                           sizeof(int) + // type
                           sizeof(int)   // recursive
    );

    test_pthread_self();
    test_pthread_create_join();
    test_pthread_mutex();

    // Must be last test.
    test_pthread_nowait();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
