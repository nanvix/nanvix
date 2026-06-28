/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <regex.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Compiles a regular expression and asserts success.
static void compile_regex(regex_t *regex, const char *pattern, int cflags)
{
    int ret;

    ret = regcomp(regex, pattern, cflags);
    assert(ret == 0);
}

// Tests required POSIX types and leftmost-longest matching.
static void test_leftmost_longest(void)
{
    regex_t regex;
    regmatch_t match[1];
    int ret;

    assert(sizeof(match[0].rm_so) >= sizeof(ptrdiff_t));
    assert(sizeof(match[0].rm_eo) >= sizeof(ptrdiff_t));

    compile_regex(&regex, "a|ab", REG_EXTENDED);
    ret = regexec(&regex, "ab", 1, match, 0);
    assert(ret == 0);
    assert(match[0].rm_so == 0);
    assert(match[0].rm_eo == 2);
    regfree(&regex);
}

// Tests REG_MINIMAL support added by POSIX Issue 8.
static void test_minimal_repetition(void)
{
    regex_t regex;
    regmatch_t match[1];
    int ret;

    compile_regex(&regex, "a.*b", REG_EXTENDED | REG_MINIMAL);
    ret = regexec(&regex, "axbyb", 1, match, 0);
    assert(ret == 0);
    assert(match[0].rm_so == 0);
    assert(match[0].rm_eo == 3);
    regfree(&regex);
}

// Tests submatch reporting and unused pmatch entries.
static void test_submatches(void)
{
    regex_t regex;
    regmatch_t match[4];
    int ret;

    compile_regex(&regex, "(a+)(b+)", REG_EXTENDED);
    assert(regex.re_nsub == 2);

    ret = regexec(&regex, "zaabbbx", 4, match, 0);
    assert(ret == 0);
    assert(match[0].rm_so == 1);
    assert(match[0].rm_eo == 6);
    assert(match[1].rm_so == 1);
    assert(match[1].rm_eo == 3);
    assert(match[2].rm_so == 3);
    assert(match[2].rm_eo == 6);
    assert(match[3].rm_so == -1);
    assert(match[3].rm_eo == -1);
    regfree(&regex);
}

// Tests REG_NOSUB and regerror() buffer sizing/truncation.
static void test_error_paths(void)
{
    regex_t regex;
    char buffer[8];
    size_t needed;
    int ret;

    compile_regex(&regex, "abc", REG_NOSUB);
    ret = regexec(&regex, "abc", 0, NULL, 0);
    assert(ret == 0);
    regfree(&regex);

    ret = regcomp(&regex, "abc\\", REG_EXTENDED);
    assert(ret == REG_EESCAPE);

    needed = regerror(REG_NOMATCH, NULL, buffer, sizeof(buffer));
    assert(needed == strlen("no match") + 1);
    assert(buffer[sizeof(buffer) - 1] == '\0');
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests POSIX regular-expression calls exposed by <regex.h>.
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

    test_leftmost_longest();
    test_minimal_repetition();
    test_submatches();
    test_error_paths();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
