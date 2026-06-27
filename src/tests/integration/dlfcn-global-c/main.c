/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-global-c: Test RTLD_GLOBAL cross-.so symbol resolution.
 *
 * This tests the plugin pattern where a host pre-loads a provider
 * library with RTLD_GLOBAL, then loads consumer plugins that reference
 * the provider's symbols without DT_NEEDED linkage.
 *
 * Per POSIX, RTLD_GLOBAL should make a library's symbols available
 * for symbol resolution of subsequently loaded libraries.
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
 * Test 1: Load libconsumer.so WITHOUT loading libprovider.so first.
 * Expected: FAIL — provider_add/mul/isqrt are unresolved.
 */
static void test_consumer_alone_fails(void)
{
    void *h = dlopen("lib/libconsumer.so", RTLD_NOW);
    if (h == NULL) {
        pass("consumer alone fails");
        return;
    }

    fail("consumer alone fails",
         "dlopen should have returned NULL for unresolved symbols");
    dlclose(h);
}

/*
 * Test 2: Load libprovider.so with RTLD_GLOBAL, then libconsumer.so.
 *
 * Per POSIX, RTLD_GLOBAL should make libprovider.so's exports
 * available to subsequently loaded libraries.
 */
static void test_global_provides_symbols(void)
{
    printf("  [2a] loading provider with RTLD_GLOBAL...\n");
    fflush(stdout);

    void *prov = dlopen("lib/libprovider.so", RTLD_GLOBAL);
    if (prov == NULL) {
        fail("RTLD_GLOBAL cross-lib", dlerror());
        return;
    }
    printf("  [2b] provider loaded, loading consumer...\n");
    fflush(stdout);

    void *cons = dlopen("lib/libconsumer.so", RTLD_NOW);
    printf("  [2c] consumer dlopen returned %p\n", (void *)cons);
    fflush(stdout);

    if (cons == NULL) {
        fail("RTLD_GLOBAL cross-lib", dlerror());
        dlclose(prov);
        return;
    }

    int (*cadd)(int, int) = NULL;
    *(void **)(&cadd) = dlsym(cons, "consumer_add");
    if (cadd == NULL || cadd(10, 20) != 30) {
        fail("RTLD_GLOBAL cross-lib", "consumer_add() failed");
        dlclose(cons);
        dlclose(prov);
        return;
    }

    dlclose(cons);
    dlclose(prov);
    pass("RTLD_GLOBAL cross-lib");
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn RTLD_GLOBAL tests ===\n");
    fflush(stdout);

    test_consumer_alone_fails();
    test_global_provides_symbols();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
