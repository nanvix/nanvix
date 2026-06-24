/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Resolved and unresolved weak cases use separate helper shared objects with
 * distinct weak symbol names so this single executable can export
 * main_callback/weak_data without also satisfying the missing-symbol cases.
 * The strong-UND regression uses RTLD_NOW so failure is asserted at dlopen(),
 * avoiding an unsafe lazy call into an unresolved strong symbol.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <dlfcn.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

int weak_data = 99;

int main_callback(int x)
{
    return (x * 10);
}

void clear_dlerror(void)
{
    (void)dlerror();
}

// Opens a dynamic load library.
void *open_library(const char *path, int flags)
{
    clear_dlerror();

    void *handle = dlopen(path, flags);
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
    clear_dlerror();

    void *sym = dlsym(handle, symbol);
    const char *error = dlerror();
    assert(error == NULL);
    assert(sym != NULL);

    return (sym);
}

// Tests if a weak undefined function resolves to the main executable.
//
// Relocation: R_386_GLOB_DAT (GOT-indirect call guarded by `if (&fn)`).
void test_weak_und_function_resolved(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*try_callback)(void) = NULL;
    *(void **)(&try_callback) = resolve_symbol(handle, "try_callback");
    assert(try_callback != NULL);
    assert(try_callback() == 70);

    close_library(handle);
}

// Tests if a missing weak undefined function resolves to zero.
//
// Relocation: R_386_GLOB_DAT (GOT-indirect call guarded by `if (&fn)`).
void test_weak_und_function_missing(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*try_callback)(void) = NULL;
    *(void **)(&try_callback) = resolve_symbol(handle, "try_callback");
    assert(try_callback != NULL);
    assert(try_callback() == -1);

    close_library(handle);
}

// Tests if weak undefined data resolves to the main executable.
//
// Relocation: R_386_GLOB_DAT (GOT-indirect load guarded by `if (&data)`).
void test_weak_und_data_resolved(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*read_weak_data)(void) = NULL;
    *(void **)(&read_weak_data) = resolve_symbol(handle, "read_weak_data");
    assert(read_weak_data != NULL);
    assert(read_weak_data() == 99);

    close_library(handle);
}

// Tests if missing weak undefined data resolves to zero.
//
// Relocation: R_386_GLOB_DAT (GOT-indirect load guarded by `if (&data)`).
void test_weak_und_data_missing(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*read_weak_data)(void) = NULL;
    *(void **)(&read_weak_data) = resolve_symbol(handle, "read_weak_data");
    assert(read_weak_data != NULL);
    assert(read_weak_data() == -1);

    close_library(handle);
}

// Tests if a weak undefined function called via PLT resolves to the main exe.
//
// Relocation: R_386_JUMP_SLOT (unguarded PLT call).
void test_weak_und_function_plt_resolved(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*call_weak_unchecked)(int) = NULL;
    *(void **)(&call_weak_unchecked) = resolve_symbol(handle, "call_weak_unchecked");
    assert(call_weak_unchecked != NULL);
    assert(call_weak_unchecked(7) == 70);

    close_library(handle);
}

// Tests that the loader accepts a .so whose weak undefined PLT slot is missing.
//
// Relocation: R_386_JUMP_SLOT (unguarded PLT call).
//
// Note: we deliberately resolve, but DO NOT call, `call_weak_unchecked` here.
// Calling it would jump through a PLT entry whose target was zeroed by the
// loader (weak undefined missing -> 0), trapping the test. The contract this
// case verifies is purely "dlopen with RTLD_NOW succeeds despite the weak
// undefined symbol", not "the call returns a value".
void test_weak_und_function_plt_missing(const char *path)
{
    void *handle = open_library(path, RTLD_NOW);

    int (*call_weak_unchecked)(int) = NULL;
    *(void **)(&call_weak_unchecked) = resolve_symbol(handle, "call_weak_unchecked");
    assert(call_weak_unchecked != NULL);

    close_library(handle);
}

// Tests if a missing strong undefined symbol still fails to load.
//
// Relocation: R_386_JUMP_SLOT against a strong UND symbol; dlopen(RTLD_NOW)
// must reject the .so at load time rather than producing a callable handle.
void test_strong_und_missing(const char *path)
{
    clear_dlerror();

    void *handle = dlopen(path, RTLD_NOW);
    const char *error = dlerror();
    assert(handle == NULL);
    assert(error != NULL);
}

/**
 * @brief Tests STB_WEAK undefined-symbol handling in the dynamic loader.
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

    // Run the strong-undefined rejection case first. It exercises a different
    // loader path (dlopen must fail outright) from the weak cases, so ordering
    // it ahead keeps a regression there reported independently of the weak ones.
    test_strong_und_missing("lib/libstrong-missing.so");
    test_weak_und_function_resolved("lib/libweak-func-resolved.so");
    test_weak_und_function_missing("lib/libweak-func-missing.so");
    test_weak_und_data_resolved("lib/libweak-data-resolved.so");
    test_weak_und_data_missing("lib/libweak-data-missing.so");
    test_weak_und_function_plt_resolved("lib/libweak-plt-resolved.so");
    test_weak_und_function_plt_missing("lib/libweak-plt-missing.so");

    // Write magic string to signal that the test passed.
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
