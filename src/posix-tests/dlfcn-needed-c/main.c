/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-needed-c: Test DT_NEEDED automatic dependency loading.
 *
 * This is the most common shared library pattern: libconsumer.so is
 * linked against libprovider.so at build time, creating a DT_NEEDED
 * entry.  When the loader opens libconsumer.so, it should automatically
 * load libprovider.so and resolve all symbols.
 *
 * This is how real-world dependencies work:
 *   - CPython C extensions have DT_NEEDED entries for libm.so, libc.so
 *   - The dynamic linker loads those automatically
 *   - No RTLD_GLOBAL or manual pre-loading is needed
 *
 * On Nanvix, this tests whether the ELF loader follows DT_NEEDED
 * entries and loads transitive dependencies.
 */

#include <assert.h>
#include <dlfcn.h>
#include <stdio.h>
#include <string.h>
#include <unistd.h>

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
 * Test 1: Load libprovider.so directly.
 * Expected: PASS (no undefined symbols).
 */
static void test_provider_loads(void)
{
    void *h = dlopen("lib/libprovider.so", RTLD_NOW);
    if (h == NULL) {
        fail("provider loads", dlerror());
        return;
    }

    int (*fn)(int, int) = NULL;
    *(void **)(&fn) = dlsym(h, "provider_add");
    if (fn == NULL || fn(2, 3) != 5) {
        fail("provider loads", "provider_add() wrong result");
        dlclose(h);
        return;
    }

    dlclose(h);
    pass("provider loads");
}

/*
 * Test 2: Load libconsumer.so (which has DT_NEEDED: libprovider.so).
 *
 * The loader should automatically load libprovider.so via DT_NEEDED,
 * resolve provider_add/mul/isqrt, and make consumer functions work.
 *
 * This is the standard pattern for how shared libraries depend on
 * each other (e.g., a .so depending on libc.so or libm.so).
 */
static void test_consumer_with_needed(void)
{
    printf("  [2a] loading consumer (has DT_NEEDED: libprovider.so)...\n");
    fflush(stdout);

    void *h = dlopen("lib/libconsumer.so", RTLD_NOW);

    printf("  [2b] dlopen returned %p\n", (void *)h);
    fflush(stdout);

    if (h == NULL) {
        fail("DT_NEEDED auto-load", dlerror());
        return;
    }

    /* Verify consumer functions work (they delegate to provider) */
    int (*cadd)(int, int) = NULL;
    *(void **)(&cadd) = dlsym(h, "consumer_add");
    if (cadd == NULL || cadd(10, 20) != 30) {
        fail("DT_NEEDED auto-load", "consumer_add() failed");
        dlclose(h);
        return;
    }

    int (*cmul)(int, int) = NULL;
    *(void **)(&cmul) = dlsym(h, "consumer_mul");
    if (cmul == NULL || cmul(6, 7) != 42) {
        fail("DT_NEEDED auto-load", "consumer_mul() failed");
        dlclose(h);
        return;
    }

    int (*cisqrt)(int) = NULL;
    *(void **)(&cisqrt) = dlsym(h, "consumer_isqrt");
    if (cisqrt == NULL || cisqrt(9) != 3) {
        fail("DT_NEEDED auto-load", "consumer_isqrt() failed");
        dlclose(h);
        return;
    }

    dlclose(h);
    pass("DT_NEEDED auto-load");
}

/*
 * Test 3: consumer_noop() has no cross-lib dependency (control test).
 */
static void test_consumer_noop(void)
{
    void *h = dlopen("lib/libconsumer.so", RTLD_LAZY);
    if (h == NULL) {
        fail("consumer_noop (no cross-lib dep)", dlerror());
        return;
    }

    int (*fn)(int) = NULL;
    *(void **)(&fn) = dlsym(h, "consumer_noop");
    if (fn == NULL || fn(41) != 42) {
        fail("consumer_noop (no cross-lib dep)", "wrong result");
        dlclose(h);
        return;
    }

    dlclose(h);
    pass("consumer_noop (no cross-lib dep)");
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn DT_NEEDED tests ===\n");
    fflush(stdout);

    test_provider_loads();
    test_consumer_with_needed();
    test_consumer_noop();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
