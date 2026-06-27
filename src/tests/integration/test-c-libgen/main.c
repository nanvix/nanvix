/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <libgen.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

static int is_either(const char *actual, const char *expected_a, const char *expected_b)
{
    return ((strcmp(actual, expected_a) == 0) || (strcmp(actual, expected_b) == 0));
}

static void test_basename(void)
{
    char usr[] = "usr";
    assert(strcmp(basename(usr), "usr") == 0);

    char usr_slash[] = "usr/";
    assert(strcmp(basename(usr_slash), "usr") == 0);

    char empty[] = "";
    assert(strcmp(basename(empty), ".") == 0);
    assert(strcmp(basename(NULL), ".") == 0);

    char root[] = "/";
    assert(strcmp(basename(root), "/") == 0);

    char double_slash[] = "//";
    assert(is_either(basename(double_slash), "/", "//"));

    char many_slashes[] = "///";
    assert(strcmp(basename(many_slashes), "/") == 0);

    char usr_lib[] = "/usr/lib";
    assert(strcmp(basename(usr_lib), "lib") == 0);

    char usr_lib_slash[] = "/usr/lib//";
    assert(strcmp(basename(usr_lib_slash), "lib") == 0);

    char repeated_slashes[] = "//usr//lib//";
    assert(strcmp(basename(repeated_slashes), "lib") == 0);

    char dot_component[] = "/home/dwc/.";
    assert(strcmp(basename(dot_component), ".") == 0);
}

static void test_dirname(void)
{
    char usr[] = "usr";
    assert(strcmp(dirname(usr), ".") == 0);

    char usr_slash[] = "usr/";
    assert(strcmp(dirname(usr_slash), ".") == 0);

    char empty[] = "";
    assert(strcmp(dirname(empty), ".") == 0);
    assert(strcmp(dirname(NULL), ".") == 0);

    char root[] = "/";
    assert(strcmp(dirname(root), "/") == 0);

    char double_slash[] = "//";
    assert(is_either(dirname(double_slash), "/", "//"));

    char many_slashes[] = "///";
    assert(is_either(dirname(many_slashes), "/", "///"));

    char usr_lib[] = "/usr/lib";
    assert(strcmp(dirname(usr_lib), "/usr") == 0);

    char usr_slash_abs[] = "/usr/";
    assert(strcmp(dirname(usr_slash_abs), "/") == 0);

    char repeated_slashes[] = "//usr//lib//";
    assert(is_either(dirname(repeated_slashes), "//usr", "/usr"));

    char nested[] = "/home//dwc//test";
    assert(is_either(dirname(nested), "/home//dwc", "/home/dwc"));

    char dotdot[] = "/home/.././test";
    assert(is_either(dirname(dotdot), "/home/../.", "/home/.."));

    char dot_component[] = "/home/dwc/.";
    assert(strcmp(dirname(dot_component), "/home/dwc") == 0);
}

/**
 * @brief Tests POSIX basename() and dirname().
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

    test_basename();
    test_dirname();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
