/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
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

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
