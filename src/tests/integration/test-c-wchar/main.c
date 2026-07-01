/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <locale.h>
#include <stddef.h>
#include <string.h>
#include <unistd.h>
#include <wchar.h>

//==================================================================================================
// Private Functions
//==================================================================================================

// Tests wcspbrk(), the wide-character "search for any of a set" function.
static void test_wcspbrk(void)
{
    // A match exists: the first wide character of "hello world" that is in "ow" is the 'o' at
    // index 4.
    {
        const wchar_t *ws1 = L"hello world";
        const wchar_t *result = wcspbrk(ws1, L"ow");
        assert(result == ws1 + 4);
        assert(*result == L'o');
    }

    // No wide character of the haystack is in the match set.
    assert(wcspbrk(L"hello", L"xyz") == NULL);

    // An empty match set never matches.
    assert(wcspbrk(L"hello", L"") == NULL);

    // The very first wide character matches.
    {
        const wchar_t *ws1 = L"abc";
        assert(wcspbrk(ws1, L"a") == ws1);
    }

    // A later wide character matches; ensure the leftmost is returned.
    {
        const wchar_t *ws1 = L"abcdef";
        const wchar_t *result = wcspbrk(ws1, L"fc");
        assert(result == ws1 + 2);
        assert(*result == L'c');
    }

    // An empty haystack never matches.
    assert(wcspbrk(L"", L"a") == NULL);
    assert(wcspbrk(L"", L"") == NULL);

    // Duplicate wide characters in the match set do not change the leftmost match.
    {
        const wchar_t *ws1 = L"abc";
        const wchar_t *result = wcspbrk(ws1, L"zzb");
        assert(result == ws1 + 1);
        assert(*result == L'b');
    }
}

// Sanity-checks the rest of the const-correct wide-search surface alongside wcspbrk so the whole
// group is covered together.
static void test_wide_search_siblings(void)
{
    const wchar_t *ws = L"hello world";

    assert(wcslen(ws) == 11);
    assert(wcschr(ws, L'o') == ws + 4);
    assert(wcsrchr(ws, L'o') == ws + 7);
    assert(wcschr(ws, L'z') == NULL);
    assert(wcsstr(ws, L"world") == ws + 6);
    assert(wcsstr(ws, L"absent") == NULL);
}

// Tests wcstof(), the wide-character to float conversion.
static void test_wcstof(void)
{
    wchar_t *end = NULL;

    // A simple value parses exactly and leaves the end pointer at the terminator.
    {
        const wchar_t *s = L"3.5";
        float v = wcstof(s, &end);
        assert(v == 3.5f);
        assert(end == s + 3);
        assert(*end == L'\0');
    }

    // A trailing suffix stops the scan and the end pointer reports the stop position.
    {
        const wchar_t *s = L"2.5x";
        float v = wcstof(s, &end);
        assert(v == 2.5f);
        assert(end == s + 3);
        assert(*end == L'x');
    }

    // A leading sign and fractional part are honored.
    assert(wcstof(L"-0.5", NULL) == -0.5f);

    // A null end pointer is accepted.
    assert(wcstof(L"10", NULL) == 10.0f);
}

// Tests wcstold(), the wide-character to long double conversion.
static void test_wcstold(void)
{
    wchar_t *end = NULL;

    // A simple value parses exactly and leaves the end pointer at the terminator.
    {
        const wchar_t *s = L"3.5";
        long double v = wcstold(s, &end);
        assert(v == 3.5L);
        assert(end == s + 3);
        assert(*end == L'\0');
    }

    // A trailing suffix stops the scan and the end pointer reports the stop position.
    {
        const wchar_t *s = L"2.5x";
        long double v = wcstold(s, &end);
        assert(v == 2.5L);
        assert(end == s + 3);
        assert(*end == L'x');
    }

    // A leading sign and fractional part are honored.
    assert(wcstold(L"-0.5", NULL) == -0.5L);

    // A null end pointer is accepted.
    assert(wcstold(L"10", NULL) == 10.0L);
}

// Tests wcscoll_l() and wcsxfrm_l(), the locale-aware wide comparison functions. Nanvix supports
// only the C/POSIX locale, so collation follows wide code-point order and the transform is the
// identity copy.
static void test_wcs_collation_l(void)
{
    locale_t loc = newlocale(LC_ALL_MASK, "C", (locale_t)0);
    assert(loc != (locale_t)0);

    // wcscoll_l() orders like wcscmp() in the C/POSIX locale.
    assert(wcscoll_l(L"abc", L"abc", loc) == 0);
    assert(wcscoll_l(L"abc", L"abd", loc) < 0);
    assert(wcscoll_l(L"abd", L"abc", loc) > 0);

    // Lowercase code points sort after uppercase ones.
    assert(wcscoll_l(L"a", L"A", loc) > 0);

    // wcsxfrm_l() performs the identity transform: the returned length excludes the terminator and
    // the destination equals the source.
    {
        wchar_t dest[8] = {L'?', L'?', L'?', L'?', L'?', L'?', L'?', L'?'};
        size_t len = wcsxfrm_l(dest, L"abc", 8, loc);
        assert(len == 3);
        assert(wcscmp(dest, L"abc") == 0);
    }

    // The defining property of wcsxfrm_l(): comparing two transformed strings with wcscmp() yields
    // the same ordering as comparing the originals with wcscoll_l().
    {
        wchar_t xa[8];
        wchar_t xb[8];
        assert(wcsxfrm_l(xa, L"abc", 8, loc) == 3);
        assert(wcsxfrm_l(xb, L"abd", 8, loc) == 3);
        assert(wcscmp(xa, xb) < 0);
    }

    // A zero-size destination writes nothing but still reports the required length.
    assert(wcsxfrm_l(NULL, L"hello", 0, loc) == 5);

    freelocale(loc);
}

// Tests mbsnrtowcs(), the bounded restartable multibyte-to-wide conversion. The C/POSIX locale maps
// each byte directly to a wide character.
static void test_mbsnrtowcs(void)
{
    // The byte budget bounds the conversion: only two of the three bytes are read, no terminator is
    // written, and src is advanced past the last converted byte.
    {
        const char in[] = "abc";
        const char *src = in;
        wchar_t out[4] = {L'?', L'?', L'?', L'?'};
        size_t n = mbsnrtowcs(out, &src, 2, 4, NULL);
        assert(n == 2);
        assert(out[0] == L'a');
        assert(out[1] == L'b');
        assert(src == in + 2);
    }

    // When the terminator falls within the budget it is converted and src is set to NULL.
    {
        const char in[] = "ab";
        const char *src = in;
        wchar_t out[4] = {L'?', L'?', L'?', L'?'};
        size_t n = mbsnrtowcs(out, &src, 8, 4, NULL);
        assert(n == 2);
        assert(out[0] == L'a');
        assert(out[1] == L'b');
        assert(out[2] == L'\0');
        assert(src == NULL);
    }

    // The destination length bounds the conversion even when byte budget remains.
    {
        const char in[] = "abc";
        const char *src = in;
        wchar_t out[1];
        size_t n = mbsnrtowcs(out, &src, 8, 1, NULL);
        assert(n == 1);
        assert(out[0] == L'a');
        assert(src == in + 1);
    }

    // A caller-supplied conversion state is accepted and round-trips to the initial state.
    {
        mbstate_t st;
        memset(&st, 0, sizeof(st));
        const char in[] = "hi";
        const char *src = in;
        wchar_t out[4] = {L'?', L'?', L'?', L'?'};
        size_t n = mbsnrtowcs(out, &src, 8, 4, &st);
        assert(n == 2);
        assert(out[0] == L'h');
        assert(out[1] == L'i');
        assert(out[2] == L'\0');
        assert(src == NULL);
        assert(mbsinit(&st) != 0);
    }

    // A null destination measures the length (bounded by the byte budget) and leaves src unchanged.
    {
        const char in[] = "abcd";
        const char *src = in;
        size_t n = mbsnrtowcs(NULL, &src, 3, 0, NULL);
        assert(n == 3);
        assert(src == in);
    }
}

// Tests wcsnrtombs(), the bounded restartable wide-to-multibyte conversion.
static void test_wcsnrtombs(void)
{
    // The wide-character budget bounds the conversion: only two of the three wide characters are
    // read, no terminator is written, and src is advanced past the last converted character.
    {
        const wchar_t in[] = L"abc";
        const wchar_t *src = in;
        char out[4] = {'?', '?', '?', '?'};
        size_t n = wcsnrtombs(out, &src, 2, 4, NULL);
        assert(n == 2);
        assert(out[0] == 'a');
        assert(out[1] == 'b');
        assert(src == in + 2);
    }

    // When the terminator falls within the budget it is converted and src is set to NULL.
    {
        const wchar_t in[] = L"ab";
        const wchar_t *src = in;
        char out[4] = {'?', '?', '?', '?'};
        size_t n = wcsnrtombs(out, &src, 8, 4, NULL);
        assert(n == 2);
        assert(out[0] == 'a');
        assert(out[1] == 'b');
        assert(out[2] == '\0');
        assert(src == NULL);
    }

    // A wide character outside the single-byte range aborts with (size_t)-1, sets errno to EILSEQ,
    // and leaves src pointing at the offending character.
    {
        const wchar_t in[] = {L'a', (wchar_t)0x100, L'\0'};
        const wchar_t *src = in;
        char out[4] = {'?', '?', '?', '?'};
        errno = 0;
        size_t n = wcsnrtombs(out, &src, 8, 4, NULL);
        assert(n == (size_t)-1);
        assert(errno == EILSEQ);
        assert(out[0] == 'a');
        assert(src == in + 1);
    }

    // A null destination measures the length (bounded by the wide-character budget) and leaves src
    // unchanged.
    {
        const wchar_t in[] = L"abcd";
        const wchar_t *src = in;
        size_t n = wcsnrtombs(NULL, &src, 2, 0, NULL);
        assert(n == 2);
        assert(src == in);
    }
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

/**
 * @brief Tests wide-character search functions in the Nanvix C library.
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
    assert(strcmp(argv[0], "test-c-wchar.elf") == 0);

    test_wcspbrk();
    test_wide_search_siblings();
    test_wcstof();
    test_wcstold();
    test_wcs_collation_l();
    test_mbsnrtowcs();
    test_wcsnrtombs();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
