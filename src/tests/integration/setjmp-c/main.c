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
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests setjmp()/longjmp() non-local control flow.
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

    test_setjmp_direct_return();
    test_longjmp_return_value();
    test_volatile_locals_preserved();
    test_jmp_buf_no_overflow();
    test_longjmp_across_calls();
    test_longjmp_retry_loop();
    test_sigsetjmp_siglongjmp();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
