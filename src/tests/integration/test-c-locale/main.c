/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <ctype.h>
#include <limits.h>
#include <locale.h>
#include <nl_types.h>
#include <stddef.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Tests the locale-management functions. Nanvix is single-locale ("C"/POSIX), so the handles are
// opaque non-null sentinels and uselocale() implements the query/swap contract.
static void test_locale_management(void)
{
    // newlocale() and duplocale() hand out non-null handles.
    locale_t loc = newlocale(LC_ALL_MASK, "C", (locale_t)0);
    assert(loc != (locale_t)0);
    locale_t dup = duplocale(loc);
    assert(dup != (locale_t)0);

    // Querying the current locale must not change it; initially it is the global locale.
    locale_t before = uselocale((locale_t)0);
    assert(uselocale((locale_t)0) == before);

    // Installing a locale returns the previous one; querying then reports the installed one.
    locale_t previous = uselocale(loc);
    assert(previous == before);
    assert(uselocale((locale_t)0) == loc);

    // Restoring the global locale returns the handle we installed.
    locale_t restored = uselocale(LC_GLOBAL_LOCALE);
    assert(restored == loc);
    assert(uselocale((locale_t)0) == LC_GLOBAL_LOCALE);

    // freelocale() accepts the handle (no-op for the static C locale).
    freelocale(dup);
    freelocale(loc);
}

// Tests the narrow per-locale ctype functions; each must agree with its non-`_l` counterpart in the
// C/POSIX locale.
static void test_ctype_l(void)
{
    locale_t loc = newlocale(LC_CTYPE_MASK, "C", (locale_t)0);

    assert(isalnum_l('a', loc) && isalnum_l('5', loc) && !isalnum_l('!', loc));
    assert(isalpha_l('z', loc) && !isalpha_l('0', loc));
    assert(isblank_l(' ', loc) && !isblank_l('x', loc));
    assert(iscntrl_l('\n', loc) && !iscntrl_l('x', loc));
    assert(isdigit_l('7', loc) && !isdigit_l('a', loc));
    assert(isgraph_l('A', loc) && !isgraph_l(' ', loc));
    assert(islower_l('a', loc) && !islower_l('A', loc));
    assert(isprint_l(' ', loc) && !isprint_l('\n', loc));
    assert(ispunct_l('!', loc) && !ispunct_l('a', loc));
    assert(isspace_l(' ', loc) && !isspace_l('a', loc));
    assert(isupper_l('A', loc) && !isupper_l('a', loc));
    assert(isxdigit_l('f', loc) && !isxdigit_l('g', loc));

    assert(tolower_l('A', loc) == 'a');
    assert(tolower_l('a', loc) == 'a');
    assert(toupper_l('a', loc) == 'A');
    assert(toupper_l('A', loc) == 'A');

    freelocale(loc);
}

// Tests the per-locale string functions strcoll_l() and strxfrm_l().
static void test_string_l(void)
{
    locale_t loc = newlocale(LC_COLLATE_MASK, "C", (locale_t)0);

    assert(strcoll_l("abc", "abd", loc) < 0);
    assert(strcoll_l("abd", "abc", loc) > 0);
    assert(strcoll_l("abc", "abc", loc) == 0);

    // In the C locale strxfrm is the identity copy and returns the source length.
    char buffer[16];
    memset(buffer, 0, sizeof(buffer));
    size_t length = strxfrm_l(buffer, "hello", sizeof(buffer), loc);
    assert(length == 5);
    assert(strcmp(buffer, "hello") == 0);

    freelocale(loc);
}

// Tests the per-locale time-formatting function strftime_l().
static void test_time_l(void)
{
    locale_t loc = newlocale(LC_TIME_MASK, "C", (locale_t)0);

    struct tm broken = {0};
    broken.tm_year = 2021 - 1900;
    broken.tm_mon = 3 - 1;
    broken.tm_mday = 14;

    char buffer[32];
    memset(buffer, 0, sizeof(buffer));
    size_t written = strftime_l(buffer, sizeof(buffer), "%Y-%m-%d", &broken, loc);
    assert(written == strlen("2021-03-14"));
    assert(strcmp(buffer, "2021-03-14") == 0);

    freelocale(loc);
}

// Tests the per-locale string-to-number conversions.
static void test_stdlib_l(void)
{
    locale_t loc = newlocale(LC_NUMERIC_MASK, "C", (locale_t)0);
    char *end;

    assert(strtod_l("3.5rest", &end, loc) == 3.5);
    assert(*end == 'r');
    assert(strtof_l("1.25", NULL, loc) == 1.25f);
    // strtold_l returns a C `long double` (80-bit x87 on both Nanvix ABIs). The
    // Rust libc has no f80 type, so on x86_64 the result comes back in the wrong
    // register class (xmm0 instead of st0) and the C caller reads garbage. Gate
    // this one conversion to i386 until the libc grows real long double support;
    // the other strto*_l conversions are ABI-portable.
#if defined(__i386__)
    assert(strtold_l("0.5", NULL, loc) == (long double)0.5);
#endif /* __i386__ */

    assert(strtol_l("-42stop", &end, 10, loc) == -42L);
    assert(*end == 's');
    assert(strtoll_l("9001", NULL, 10, loc) == 9001LL);
    assert(strtoul_l("100", NULL, 10, loc) == 100UL);
    assert(strtoull_l("0xff", NULL, 16, loc) == 255ULL);

    freelocale(loc);
}

// Tests the XSI message-catalog functions. Nanvix supports only the C/POSIX locale, which
// defines no message catalogs, so catopen() reports failure, catgets() echoes the fallback
// string, and catclose() succeeds.
static void test_message_catalog(void)
{
    // catopen() cannot find any catalog and reports failure with the (nl_catd)-1 sentinel.
    nl_catd catd = catopen("messages", NL_CAT_LOCALE);
    assert(catd == (nl_catd)-1);

    // catgets() returns the caller-supplied fallback string unchanged (same pointer).
    const char *fallback = "fallback message";
    assert((const char *)catgets(catd, NL_SETD, 1, fallback) == fallback);

    // catclose() always succeeds.
    assert(catclose(catd) == 0);
}

// Tests localeconv(): in the C/POSIX locale the decimal point is ".", every other string field is
// empty, and every numeric field is CHAR_MAX ("not available"). This includes the six C99
// international monetary members (int_p_cs_precedes and friends).
static void test_localeconv(void)
{
    struct lconv *lc = localeconv();
    assert(lc != NULL);

    // Only the decimal point is non-empty in the C locale; all other string fields are empty.
    assert(strcmp(lc->decimal_point, ".") == 0);
    assert(strcmp(lc->thousands_sep, "") == 0);
    assert(strcmp(lc->grouping, "") == 0);
    assert(strcmp(lc->int_curr_symbol, "") == 0);
    assert(strcmp(lc->currency_symbol, "") == 0);
    assert(strcmp(lc->mon_decimal_point, "") == 0);
    assert(strcmp(lc->mon_thousands_sep, "") == 0);
    assert(strcmp(lc->mon_grouping, "") == 0);
    assert(strcmp(lc->positive_sign, "") == 0);
    assert(strcmp(lc->negative_sign, "") == 0);

    // Local numeric and monetary members are all CHAR_MAX in the C locale.
    assert(lc->int_frac_digits == CHAR_MAX);
    assert(lc->frac_digits == CHAR_MAX);
    assert(lc->p_cs_precedes == CHAR_MAX);
    assert(lc->p_sep_by_space == CHAR_MAX);
    assert(lc->n_cs_precedes == CHAR_MAX);
    assert(lc->n_sep_by_space == CHAR_MAX);
    assert(lc->p_sign_posn == CHAR_MAX);
    assert(lc->n_sign_posn == CHAR_MAX);

    // C99 international monetary members.
    assert(lc->int_p_cs_precedes == CHAR_MAX);
    assert(lc->int_p_sep_by_space == CHAR_MAX);
    assert(lc->int_n_cs_precedes == CHAR_MAX);
    assert(lc->int_n_sep_by_space == CHAR_MAX);
    assert(lc->int_p_sign_posn == CHAR_MAX);
    assert(lc->int_n_sign_posn == CHAR_MAX);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests the POSIX xlocale API (locale management plus the narrow `*_l` functions).
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
    assert(strcmp(argv[0], "test-c-locale.elf") == 0);

    test_locale_management();
    test_ctype_l();
    test_string_l();
    test_time_l();
    test_stdlib_l();
    test_message_catalog();
    test_localeconv();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
