/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <sys/utsname.h>
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

// Tests if uname()  works.
void test_uname(void)
{
    // Sanity check size of `utsname` structure.
    STATIC_ASSERT_SIZE(struct utsname,
                       _UTSNAME_LENGTH * sizeof(char) +     // sysname
                           _UTSNAME_LENGTH * sizeof(char) + // nodename
                           _UTSNAME_LENGTH * sizeof(char) + // release
                           _UTSNAME_LENGTH * sizeof(char) + // version
                           _UTSNAME_LENGTH * sizeof(char)   // machine
    );

    // Get system information.
    struct utsname utsname = {0};
    assert(uname(&utsname) == 0);

    // Check if the system information structure is not empty.
    assert(utsname.sysname[0] != '\0');
    assert(utsname.nodename[0] != '\0');
    assert(utsname.release[0] != '\0');
    assert(utsname.version[0] != '\0');
    assert(utsname.machine[0] != '\0');
}

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
    // TODO: assert that argv[0] is the name of the executable.

    test_uname();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
