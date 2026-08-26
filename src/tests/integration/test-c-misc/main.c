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
#include <string.h>
#include <sys/utsname.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests miscellaneous system calls.
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

    // Assert command-line arguments.
    assert(argc == 1);
    assert(argv[0] != NULL);
    assert(argv[1] == NULL);
    assert(strcmp(argv[0], "test-c-misc.elf") == 0);

    test_getuid();
    test_getgid();
    test_geteuid();
    test_getegid();
    test_setuid();
    test_seteuid();
    test_setgid();
    test_setegid();
    test_setresuid();
    test_setresgid();
    test_setgroups();
    test_initgroups();
    test_user_group();
    test_clock_getres();
    test_clock_gettime();
#ifndef __hyperlight__
    test_nanosleep();
    test_sleep();
#endif
    test_times();
    test_uname();
    test_gethostname();
    test_getenv();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
