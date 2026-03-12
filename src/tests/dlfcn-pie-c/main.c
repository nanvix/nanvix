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

// A global variable exported by the main executable (via -rdynamic).
int exe_global_value = 42;

// Tests if dlopen(NULL) returns a handle to the global symbol scope and
// dlsym() can resolve symbols exported by the main executable.
void test_dlopen_null(void)
{
    // dlopen(NULL) should return a valid handle per POSIX.
    void *handle = dlopen(NULL, RTLD_NOW);
    assert(handle != NULL);

    // Resolve a symbol that the main executable exports.
    int *value = (int *)dlsym(handle, "exe_global_value");
    assert(value != NULL);
    assert(*value == 42);

    // RTLD_DEFAULT (NULL handle) should also resolve the same symbol.
    int *value2 = (int *)dlsym(RTLD_DEFAULT, "exe_global_value");
    assert(value2 != NULL);
    assert(*value2 == 42);
    assert(value == value2);

    // Closing the global handle should be a no-op.
    assert(dlclose(handle) == 0);
}

// Tests if dlsym() works on a shared library loaded from a PIE executable.
void test_dlsym(const char *path)
{
    void *handle = open_library(path);

    int (*add)(int, int) = NULL;
    *(void **)(&add) = resolve_symbol(handle, "add");
    assert(add != NULL);
    assert(add(1, 2) == 3);

    int (*multiply)(int, int) = NULL;
    *(void **)(&multiply) = resolve_symbol(handle, "multiply");
    assert(multiply != NULL);
    assert(multiply(7, 6) == 42);

    const char **version = resolve_symbol(handle, "VERSION");
    assert(version != NULL);
    assert(!strcmp(*version, "0.0.1"));

    close_library(handle);
}

/**
 * @brief Tests dynamic loading from a PIE executable with exported symbols.
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

    test_dlopen_null();
    test_dlsym("lib/libmul-pie.so");

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 3);
    }

    return (0);
}
