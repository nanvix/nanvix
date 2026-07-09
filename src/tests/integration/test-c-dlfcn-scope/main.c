/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-scope-c: Validate that dlsym(handle, name) searches ONLY the object's
 * load group -- the library itself plus its DT_NEEDED dependencies -- and never
 * falls back to the global symbol scope. Per POSIX, the global scope is
 * reachable only through the RTLD_DEFAULT / dlopen(NULL) pseudo-handle.
 *
 * Regression coverage: dlsym(handle, ...) must not search the global symbol
 * table when a symbol is absent from the handle's load group. Two distinct
 * sources populate the global scope, and BOTH must stay
 * invisible to a specific handle:
 *
 *   1. The main executable's own --export-dynamic symbols (seeded into the
 *      loader's global scope at startup by dlinit()).
 *   2. Symbols of any library dlopen()'d with RTLD_GLOBAL.
 *
 * Fixtures (staged under lib/ by the per-suite RAMFS image):
 *   - libdep.so   exports scope_dep_value(); self-contained.
 *   - libfoo.so   exports scope_foo_value(); DT_NEEDED on libdep.so and calls
 *                 scope_dep_value(), so libdep.so is part of libfoo.so's load
 *                 group. It neither defines nor references scope_main_export()
 *                 (main exe) or scope_other_value() (libother.so).
 *   - libother.so exports scope_other_value(); self-contained and unrelated to
 *                 libfoo.so (no dependency edge in either direction).
 *
 * The main executable is linked PIE + --export-dynamic so scope_main_export()
 * lands in .dynsym and is registered in the loader's global scope. Because
 * libfoo.so has no dependency path to either scope_main_export() (main exe) or
 * scope_other_value() (an RTLD_GLOBAL library), a non-NULL result from
 * dlsym(libfoo_handle, ...) for those names could ONLY come from an incorrect
 * global fallback -- exactly the bug this suite guards against. The load-group
 * probes (scope_foo_value / scope_dep_value) are the complementary regression
 * check that the fix did not break legitimate self + DT_NEEDED resolution.
 *
 * Pass/fail is signalled by the exit code (0 = pass), which nanvixd
 * propagates; the trailing "ok" write mirrors the other suites' magic marker.
 */

#include <dlfcn.h>
#include <stdio.h>
#include <unistd.h>

/* Sentinel return values, distinct so a mis-resolved pointer is obvious. Each
 * must match the literal returned by the corresponding fixture. */
#define MAIN_EXPORT_VALUE 2130          /* scope_main_export() (this file)  */
#define DEP_VALUE 111                   /* scope_dep_value()   (libdep.so)  */
#define FOO_VALUE (DEP_VALUE + 100)     /* scope_foo_value()   (libfoo.so)  */
#define OTHER_VALUE 333                 /* scope_other_value() (libother.so)*/

/*
 * Exported by the main executable via --export-dynamic. Registered in the
 * loader's global scope at startup, so it is reachable through RTLD_DEFAULT but
 * must NOT be reachable through a specific library handle. Marked `used` so the
 * compiler emits it even though nothing references it at link time.
 */
__attribute__((used)) int scope_main_export(void)
{
    return MAIN_EXPORT_VALUE;
}

/* Function-pointer shape of every probed symbol. */
typedef int (*int_fn)(void);

static int tests_passed = 0;
static int tests_failed = 0;

static void pass(const char *name)
{
    printf("  PASS: %s\n", name);
    fflush(stdout);
    tests_passed++;
}

static void fail(const char *name, const char *reason)
{
    /* dlerror() may return NULL (no diagnostic available); guard against
     * passing a NULL pointer to printf("%s"), which is undefined behavior. */
    printf("  FAIL: %s (%s)\n", name, reason != NULL ? reason : "no diagnostic available");
    fflush(stdout);
    tests_failed++;
}

/*
 * Precondition: scope_main_export() really is in the loader's global scope, so
 * the "hidden from handle" probe below is meaningful. RTLD_DEFAULT searches the
 * default (global) scope, so it must find and correctly call the symbol.
 */
static void test_main_export_visible_via_default(void)
{
    const char *name = "main-exe --export-dynamic symbol is visible via RTLD_DEFAULT";

    (void)dlerror();
    int_fn fn = (int_fn)dlsym(RTLD_DEFAULT, "scope_main_export");
    if (fn == NULL) {
        fail(name, "scope_main_export absent from global scope (fixture/link error)");
        return;
    }
    if (fn() != MAIN_EXPORT_VALUE) {
        fail(name, "scope_main_export resolved to the wrong function");
        return;
    }

    pass(name);
}

/*
 * Primary probe: the main executable's global-scope symbol must NOT be
 * reachable through libfoo.so's handle. libfoo.so does not define or depend on
 * scope_main_export(), so a non-NULL result here is the global-fallback bug.
 */
static void test_main_export_hidden_from_handle(void *foo)
{
    const char *name = "main-exe global symbol is hidden from dlsym(handle, ...)";

    (void)dlerror();
    void *p = dlsym(foo, "scope_main_export");
    if (p != NULL) {
        fail(name, "global-scope symbol leaked into handle lookup");
        return;
    }

    /* A genuine load-group search must record a diagnostic on failure; a stub
     * that unconditionally returns NULL would leave dlerror() unset. */
    if (dlerror() == NULL) {
        fail(name, "missing-symbol handle lookup did not set dlerror()");
        return;
    }

    pass(name);
}

/*
 * Regression: dlsym(handle, ...) still resolves the object's OWN exported
 * symbol (step 1 of the load-group search).
 */
static void test_handle_resolves_self(void *foo)
{
    const char *name = "dlsym(handle, ...) resolves the object's own symbol";

    (void)dlerror();
    int_fn fn = (int_fn)dlsym(foo, "scope_foo_value");
    if (fn == NULL) {
        fail(name, "own symbol not found in the load group");
        return;
    }
    if (fn() != FOO_VALUE) {
        fail(name, "own symbol resolved incorrectly");
        return;
    }

    pass(name);
}

/*
 * Regression: dlsym(handle, ...) still resolves a symbol defined in a
 * DT_NEEDED dependency (step 2 of the load-group search). scope_dep_value()
 * lives in libdep.so, which is part of libfoo.so's load group.
 */
static void test_handle_resolves_dependency(void *foo)
{
    const char *name = "dlsym(handle, ...) resolves a DT_NEEDED dependency symbol";

    (void)dlerror();
    int_fn fn = (int_fn)dlsym(foo, "scope_dep_value");
    if (fn == NULL) {
        fail(name, "dependency symbol not found in the load group");
        return;
    }
    if (fn() != DEP_VALUE) {
        fail(name, "dependency symbol resolved incorrectly");
        return;
    }

    pass(name);
}

/*
 * Secondary probe: a symbol published into the global scope by a library
 * loaded with RTLD_GLOBAL must be visible via RTLD_DEFAULT yet remain invisible
 * to an UNRELATED handle (libfoo.so), which has no dependency edge to it.
 */
static void test_rtld_global_hidden_from_handle(void *foo)
{
    const char *name = "RTLD_GLOBAL symbol is hidden from an unrelated handle";

    void *other = dlopen("lib/libother.so", RTLD_NOW | RTLD_GLOBAL);
    if (other == NULL) {
        fail(name, dlerror());
        return;
    }

    /* Now in the global scope: reachable via RTLD_DEFAULT ... */
    (void)dlerror();
    int_fn via_default = (int_fn)dlsym(RTLD_DEFAULT, "scope_other_value");
    if (via_default == NULL || via_default() != OTHER_VALUE) {
        fail(name, "RTLD_GLOBAL symbol not visible via RTLD_DEFAULT");
        dlclose(other);
        return;
    }

    /* ... but NOT through libfoo.so's handle (no dependency edge to it). */
    (void)dlerror();
    void *via_handle = dlsym(foo, "scope_other_value");
    if (via_handle != NULL) {
        fail(name, "RTLD_GLOBAL symbol leaked into unrelated handle lookup");
        dlclose(other);
        return;
    }

    /* A genuine load-group search must record a diagnostic on failure; a stub
     * that unconditionally returns NULL would leave dlerror() unset. */
    if (dlerror() == NULL) {
        fail(name, "missing-symbol handle lookup did not set dlerror()");
        dlclose(other);
        return;
    }

    if (dlclose(other) != 0) {
        fail(name, "dlclose(lib/libother.so) failed");
        return;
    }

    pass(name);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn dlsym(handle) load-group scope tests ===\n");
    fflush(stdout);

    test_main_export_visible_via_default();

    void *foo = dlopen("lib/libfoo.so", RTLD_NOW);
    if (foo == NULL) {
        fail("dlopen(lib/libfoo.so)", dlerror());
    } else {
        test_main_export_hidden_from_handle(foo);
        test_handle_resolves_self(foo);
        test_handle_resolves_dependency(foo);
        test_rtld_global_hidden_from_handle(foo);

        if (dlclose(foo) != 0) {
            fail("dlclose(lib/libfoo.so)", "dlclose failed");
        }
    }

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
        return 0;
    }

    return 1;
}
