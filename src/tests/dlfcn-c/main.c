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

// Opens a dynamic load library.
void *open_library(const char *path)
{
    void *handle = dlopen(path, RTLD_LAZY);
    assert(handle != NULL);
    return (handle);
}

// Closes a dynamic load library.
void close_library(void *handle)
{
    assert(dlclose(handle) == 0);
}

// Resolves a symbol in a dynamic load library.
void *resolve_symbol(void *handle, const char *symbol)
{
    void *sym = dlsym(handle, symbol);
    assert(sym != NULL);
    return (sym);
}

// Tests if dlopen() and dlclose() work.
void test_dlopen_dlclose(const char *path)
{
    void *handle = open_library(path);
    close_library(handle);
}

// Tests if dlsym() works.
void test_dlsym(const char *path)
{
    void *handle = open_library(path);

    int (*add)(int, int) = NULL;
    *(void **)(&add) = resolve_symbol(handle, "add");
    assert(add != NULL);
    assert(add(1, 2) == 3);

    int (*fast_mul)(int, int) = NULL;
    *(void **)(&fast_mul) = resolve_symbol(handle, "fast_mul");
    assert(fast_mul != NULL);
    assert(fast_mul(7, 2) == 14);

    int (*slow_mul)(int, int) = NULL;
    *(void **)(&slow_mul) = resolve_symbol(handle, "slow_mul");
    assert(slow_mul != NULL);
    assert(slow_mul(3, 4) == 12);

    int (*multiply)(int, int) = NULL;
    *(void **)(&multiply) = resolve_symbol(handle, "multiply");
    assert(multiply != NULL);
    assert(multiply(7, 6) == 42);

    const char **version = resolve_symbol(handle, "VERSION");
    assert(version != NULL);
    assert(!strcmp(*version, "0.0.1"));

    const char *(*get_version)(void) = NULL;
    *(void **)(&get_version) = resolve_symbol(handle, "get_version");
    assert(get_version != NULL);
    assert(!strcmp(get_version(), "0.0.1"));

    close_library(handle);
}

// Tests if dladdr() works.
void test_dladdr(const char *path)
{
    void *handle = open_library(path);

    int (*add)(int, int) = NULL;
    *(void **)(&add) = resolve_symbol(handle, "add");
    assert(add != NULL);

    void *add_addr = *(void **)(&add);

    Dl_info_t info;
    assert(dladdr(add_addr, &info) == 0);
    assert(!strcmp(info.dli_fname, path));
    assert(info.dli_fbase != NULL);
    assert(!strcmp(info.dli_sname, "add"));
    assert(info.dli_saddr == add_addr);

    close_library(handle);
}

/**
 * @brief Tests system calls to operate on dynamic load libraries.
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

    // Assert types in <dlfcn.h>.
    STATIC_ASSERT_SIZE(Dl_info_t,
                       sizeof(char *)             // dli_fname
                           + sizeof(void *)       // dli_fbase
                           + sizeof(const char *) // dli_sname
                           + sizeof(void *)       // dli_saddr
    );

    test_dlopen_dlclose("lib/libmul.so");
    test_dlsym("lib/libmul.so");
    test_dladdr("lib/libmul.so");

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
