/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <dlfcn.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

enum {
    STAGED_CONSTRUCTOR_VALUE = 0x5a61,
    GOOD_ROOT_VALUE = STAGED_CONSTRUCTOR_VALUE + 1,
};

//==================================================================================================
// Standalone Functions
//==================================================================================================

static void clear_dlerror(void)
{
    (void)dlerror();
}

static void *open_library(const char *path)
{
    clear_dlerror();

    void *handle = dlopen(path, RTLD_NOW);
    assert(handle != NULL);
    assert(dlerror() == NULL);

    return (handle);
}

static int (*resolve_value(void *handle, const char *name))(void)
{
    clear_dlerror();

    int (*value)(void) = NULL;
    *(void **)(&value) = dlsym(handle, name);
    assert(value != NULL);
    assert(dlerror() == NULL);

    return (value);
}

static void test_failed_load_discards_staging(void)
{
    clear_dlerror();

    void *handle = dlopen("lib/libfailed-root.so", RTLD_NOW);
    assert(handle == NULL);
    assert(dlerror() != NULL);

    handle = open_library("lib/libstaged.so");
    int (*staged_value)(void) = resolve_value(handle, "staged_value");
    assert(staged_value() == STAGED_CONSTRUCTOR_VALUE);

    void *root = open_library("lib/libgood-root.so");
    int (*good_root_value)(void) = resolve_value(root, "good_root_value");
    assert(good_root_value() == GOOD_ROOT_VALUE);

    assert(dlclose(root) == 0);
    assert(dlclose(handle) == 0);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    test_failed_load_discards_staging();

    const char *magic = "ok";
    write(STDOUT_FILENO, magic, 2);

    return (0);
}
