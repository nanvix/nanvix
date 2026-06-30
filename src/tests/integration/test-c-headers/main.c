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
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Exercises the `restrict`-qualified <stdlib.h> string-to-number conversions. These prototypes
// carry the bare C99 `restrict` keyword that the C++-safety header fix neutralizes under C++; in C
// the keyword stays, so this confirms the regenerated declarations still parse and behave correctly
// when compiled as C.
static void test_restrict_conversions(void)
{
    char *end;

    end = NULL;
    assert(strtod("  12.5 rest", &end) == 12.5);
    assert(end != NULL && *end == ' ');

    end = NULL;
    assert(strtof("3.25xyz", &end) == 3.25f);
    assert(*end == 'x');

    end = NULL;
    assert(strtold("0.5!", &end) == (long double)0.5);
    assert(*end == '!');

    end = NULL;
    assert(strtol("  -42stop", &end, 10) == -42L);
    assert(*end == 's');

    end = NULL;
    assert(strtoll("9001;", &end, 10) == 9001LL);
    assert(*end == ';');

    end = NULL;
    assert(strtoul("100/", &end, 10) == 100UL);
    assert(*end == '/');

    end = NULL;
    assert(strtoull("0xff?", &end, 16) == 255ULL);
    assert(*end == '?');
}

// Exercises <regex.h>, whose `regcomp`/`regexec`/`regerror` prototypes carry bare `restrict`
// (including the array-parameter form `regmatch_t pmatch[restrict]`). A compile + match round-trip
// confirms the regenerated header is usable from C after the C++-safety fix.
static void test_regex_restrict_params(void)
{
    regex_t regex;
    regmatch_t match[1];

    assert(regcomp(&regex, "a.c", REG_EXTENDED) == 0);
    assert(regexec(&regex, "xabcy", 1, match, 0) == 0);
    assert(match[0].rm_so == 1);
    assert(match[0].rm_eo == 4);
    regfree(&regex);
}

// Validates that the declarations whose parameter names were sanitized away from C++ keywords
// (`new` -> `newpath` in rename(); `template` -> `tmpl` in the temporary-file creators) are still
// present with their expected C signatures and resolve at link time. The addresses are kept
// live through volatile pointers so the references are not optimized away; no filesystem side
// effects are performed.
static void test_sanitized_parameter_decls(void)
{
    int (*volatile rename_fn)(const char *, const char *) = rename;
    char *(*volatile mkdtemp_fn)(char *) = mkdtemp;
    int (*volatile mkostemp_fn)(char *, int) = mkostemp;
    int (*volatile mkstemp_fn)(char *) = mkstemp;
    char *(*volatile mktemp_fn)(char *) = mktemp;

    assert(rename_fn != NULL);
    assert(mkdtemp_fn != NULL);
    assert(mkostemp_fn != NULL);
    assert(mkstemp_fn != NULL);
    assert(mktemp_fn != NULL);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests that the generated libc headers expose correct C declarations after the C++-safety
 * regeneration.
 *
 * @param argc Number of command-line arguments.
 * @param argv List of command-line arguments.
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
    assert(strcmp(argv[0], "test-c-headers.elf") == 0);

    test_restrict_conversions();
    test_regex_restrict_params();
    test_sanitized_parameter_decls();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
