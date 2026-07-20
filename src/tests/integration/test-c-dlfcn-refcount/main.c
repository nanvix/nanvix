/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <dlfcn.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

/**
 * @brief Shared library exercised by this suite.
 *
 * The prebuilt `libmul.so` fixture (staged under `lib/` by the shared POSIX
 * tests RAMFS image) exports self-contained `add()` and `multiply()` functions
 * and carries no dependencies, so it is a clean subject for reference-counting
 * checks.
 */
#define LIB_PATH "lib/libmul.so"

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Opens a dynamic load library, requesting immediate binding.
static void *open_library(const char *path)
{
    void *handle = dlopen(path, RTLD_NOW);
    assert(handle != NULL);
    return (handle);
}

// Closes a dynamic load library.
static void close_library(void *handle)
{
    assert(dlclose(handle) == 0);
}

// Resolves a symbol in a dynamic load library.
static void *resolve_symbol(void *handle, const char *symbol)
{
    void *sym = dlsym(handle, symbol);
    assert(sym != NULL);
    return (sym);
}

// Resolves `add` through `handle` and asserts that the still-mapped code runs.
static void assert_library_usable(void *handle)
{
    int (*add)(int, int) = NULL;
    *(void **)(&add) = resolve_symbol(handle, "add");
    assert(add != NULL);
    assert(add(1, 2) == 3);

    int (*multiply)(int, int) = NULL;
    *(void **)(&multiply) = resolve_symbol(handle, "multiply");
    assert(multiply != NULL);
    assert(multiply(7, 6) == 42);
}

//==================================================================================================
// Test Functions
//==================================================================================================

/**
 * @brief Regression test: a staggered open/close must not unload a library that
 * still has a live handle.
 *
 * Each dlopen() of an already-loaded library is a cache hit that returns the
 * same handle, so the loader must track an explicit open count rather than an
 * internal Arc reference count. Opening twice and closing once must leave the
 * library mapped; the follow-up dlsym()/call would fault or fail if it had been
 * wrongly unloaded.
 */
static void test_staggered_open_close(void)
{
    void *h1 = open_library(LIB_PATH);
    void *h2 = open_library(LIB_PATH);

    // A second dlopen() of the same file is a cache hit: it returns the same
    // handle and bumps the open count to two.
    assert(h1 == h2);

    // First dlclose() drops the open count to one; the library must stay loaded.
    close_library(h1);

    // The surviving handle must still resolve and run code from the still-mapped
    // library. On the pre-fix loader the library would already be unloaded here.
    assert_library_usable(h2);

    // Final dlclose() drops the open count to zero and unloads the library.
    close_library(h2);
}

/**
 * @brief Verifies that the open count balances correctly across more than two
 * opens.
 *
 * Opening three times and closing twice must keep the library loaded; only the
 * third close (open count reaching zero) may unload it.
 */
static void test_balanced_open_close(void)
{
    void *h1 = open_library(LIB_PATH);
    void *h2 = open_library(LIB_PATH);
    void *h3 = open_library(LIB_PATH);

    // Every cache hit resolves to the same handle.
    assert(h1 == h2);
    assert(h2 == h3);

    // Two closes still leave one outstanding open.
    close_library(h1);
    assert_library_usable(h2);
    close_library(h2);
    assert_library_usable(h3);

    // The third close balances the last open and unloads the library.
    close_library(h3);
}

/**
 * @brief Verifies that a fresh dlopen() after a full close reloads the library.
 *
 * Once every handle is closed the library is unloaded; a subsequent dlopen()
 * must succeed and yield a usable library again.
 */
static void test_reopen_after_full_close(void)
{
    void *h1 = open_library(LIB_PATH);
    assert_library_usable(h1);
    close_library(h1);

    void *h2 = open_library(LIB_PATH);
    assert_library_usable(h2);
    close_library(h2);
}

/**
 * @brief Tests reference counting of dlopen()/dlclose().
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

    test_staggered_open_close();
    test_balanced_open_close();
    test_reopen_after_full_close();

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
