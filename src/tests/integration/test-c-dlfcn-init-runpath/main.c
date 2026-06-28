/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-init-runpath-c: Tests for `.init_array` / `.fini_array`
 * constructor and destructor invocation, and for DT_RUNPATH-driven
 * dependency search.
 *
 * Test 1 (constructor): dlopen libctor.so, dlsym `ctor_ran`, and
 *   confirm the constructor sentinel was written.
 *
 * Test 2 (destructor): dlclose libctor.so and confirm the destructor
 *   wrote its sentinel into the main executable's `g_dtor_ran` global.
 *   `g_dtor_ran` must be exported via --export-dynamic so the loader's
 *   global symbol table can satisfy the `extern volatile int g_dtor_ran;`
 *   reference in the library.
 *
 * Test 3 (DT_RUNPATH): dlopen lib/libparent.so, which has
 *   DT_NEEDED=libchild.so and DT_RUNPATH=lib/subdir/. The loader must
 *   probe DT_RUNPATH before the default `lib/` directory; otherwise
 *   libchild.so is not located.
 */

#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

/* Witness for libctor.so's destructor -- defined here so it survives the
 * library being unloaded. Marked volatile to keep the optimiser away.
 */
volatile int g_dtor_ran = 0;

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
    printf("  FAIL: %s (%s)\n", name, reason);
    fflush(stdout);
    tests_failed++;
}

/*
 * Test 1 (constructor) + Test 2 (destructor): the constructor in
 * `.init_array` must run before dlopen returns, and the destructor in
 * `.fini_array` must run on dlclose. Both are checked here so the
 * destructor witness (`g_dtor_ran`) is observed while it is fresh.
 */
static void test_init_array(void)
{
    void *h = dlopen("lib/libctor.so", RTLD_NOW);
    if (h == NULL) {
        fail("init_array fires on dlopen", dlerror());
        return;
    }

    volatile int *ctor_ran = (volatile int *)dlsym(h, "ctor_ran");
    if (ctor_ran == NULL) {
        fail("init_array fires on dlopen", "ctor_ran symbol missing");
        dlclose(h);
        return;
    }

    if (*ctor_ran != 0xC70A) {
        fail("init_array fires on dlopen", "constructor sentinel not set");
        dlclose(h);
        return;
    }

    /* Sanity check: ordinary symbol resolution still works after init. */
    int (*fn)(void) = NULL;
    *(void **)(&fn) = dlsym(h, "ctor_value");
    if (fn == NULL || fn() != 42) {
        fail("init_array fires on dlopen", "ctor_value() wrong");
        dlclose(h);
        return;
    }

    /* Reset the dtor witness so test_fini_array measures a fresh signal. */
    g_dtor_ran = 0;
    if (dlclose(h) != 0) {
        fail("fini_array fires on dlclose", dlerror());
        return;
    }

    pass("init_array fires on dlopen");

    /* Bridge directly into the destructor test while the witness is fresh. */
    if (g_dtor_ran != 0xD70A) {
        fail("fini_array fires on dlclose", "destructor sentinel not set");
        return;
    }
    pass("fini_array fires on dlclose");
}

/*
 * Test 3: DT_RUNPATH must be consulted when resolving DT_NEEDED bare
 * names. libparent.so depends on libchild.so but libchild.so only
 * exists under lib/subdir/, which is libparent's DT_RUNPATH.
 */
static void test_dt_runpath(void)
{
    void *h = dlopen("lib/libparent.so", RTLD_NOW);
    if (h == NULL) {
        fail("DT_RUNPATH dependency search", dlerror());
        return;
    }

    int (*fn)(int) = NULL;
    *(void **)(&fn) = dlsym(h, "parent_value");
    if (fn == NULL || fn(5) != 24) {
        /* parent_value(5) = child_value(5) * 2 = (5 + 7) * 2 = 24 */
        fail("DT_RUNPATH dependency search", "parent_value() wrong");
        dlclose(h);
        return;
    }

    if (dlclose(h) != 0) {
        fail("DT_RUNPATH dependency search", dlerror());
        return;
    }
    pass("DT_RUNPATH dependency search");
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn init_array + DT_RUNPATH tests ===\n");
    fflush(stdout);

    test_init_array();
    test_dt_runpath();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
