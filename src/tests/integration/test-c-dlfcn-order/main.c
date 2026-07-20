/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * dlfcn-order-c: acceptance test for DT_NEEDED dependency search ORDER
 * (nanvix/nanvix#2091).
 *
 *   libroot.so
 *     +-- DT_NEEDED libbravo.so   (provider_id() == 2)  [first  in DT_NEEDED]
 *     +-- DT_NEEDED libalpha.so   (provider_id() == 1)  [second in DT_NEEDED]
 *
 * Both dependencies export the SAME symbol `provider_id()`. POSIX/System V (and
 * glibc, via its `l_searchlist`) resolve a symbol looked up through an object to
 * the first definition found while searching that object's dependencies in
 * DT_NEEDED order. libroot.so's DT_NEEDED order is (libbravo.so, libalpha.so),
 * which is the REVERSE of the alphabetical order of the names -- so:
 *
 *   * a DT_NEEDED-order (BFS) loader resolves `provider_id()` to libbravo.so's
 *     definition, returning 2;
 *   * a loader that searches dependencies alphabetically (the pre-fix bug,
 *     where dependencies were stored in a `BTreeMap` keyed by name) resolves it
 *     to libalpha.so's definition, returning 1.
 *
 * The decisive assertions below expect 2. On the buggy loader they observe 1 and
 * the test fails, so this suite is a genuine regression test for the fix.
 */

#include <dlfcn.h>
#include <stdio.h>
#include <unistd.h>

/* Identity each provider returns from provider_id(). */
#define ALPHA_ID 1
#define BRAVO_ID 2

/* Per-library unique marker values. */
#define ALPHA_MARK 0xA
#define BRAVO_MARK 0xB

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
 * Sanity: both dependencies are actually loaded and reachable through
 * libroot.so's handle. Each marker is unique to one library, so this does not
 * depend on search order -- it just confirms the test setup is sound before the
 * order-sensitive assertions run.
 */
static void test_both_dependencies_loaded(void *h)
{
    int (*root_touch)(void) = (int (*)(void))dlsym(h, "root_touch");
    int (*alpha_marker)(void) = (int (*)(void))dlsym(h, "alpha_marker");
    int (*bravo_marker)(void) = (int (*)(void))dlsym(h, "bravo_marker");

    if (!root_touch || !alpha_marker || !bravo_marker) {
        fail("both dependencies loaded", "missing marker symbols");
        return;
    }
    if (alpha_marker() != ALPHA_MARK) {
        fail("both dependencies loaded", "libalpha.so not reachable via handle");
        return;
    }
    if (bravo_marker() != BRAVO_MARK) {
        fail("both dependencies loaded", "libbravo.so not reachable via handle");
        return;
    }
    if (root_touch() != ALPHA_MARK + BRAVO_MARK) {
        fail("both dependencies loaded", "libroot.so relocations not bound to both deps");
        return;
    }
    pass("both dependencies loaded and reachable");
}

/*
 * KEY TEST (dlsym path): dlsym(libroot_handle, "provider_id") must resolve to
 * the FIRST definition in DT_NEEDED order -- libbravo.so's (id 2) -- not the
 * alphabetically-first one -- libalpha.so's (id 1).
 */
static void test_dlsym_resolves_in_dt_needed_order(void *h)
{
    int (*provider_id)(void) = (int (*)(void))dlsym(h, "provider_id");
    if (!provider_id) {
        fail("dlsym resolves in DT_NEEDED order", dlerror());
        return;
    }

    int id = provider_id();
    if (id != BRAVO_ID) {
        char reason[128];
        snprintf(reason, sizeof(reason),
                 "provider_id()=%d (expected %d from libbravo.so, the first "
                 "DT_NEEDED entry); got libalpha.so (%d) -- dependencies searched "
                 "alphabetically instead of in DT_NEEDED order",
                 id, BRAVO_ID, ALPHA_ID);
        fail("dlsym resolves in DT_NEEDED order", reason);
        return;
    }
    pass("dlsym resolves provider_id() in DT_NEEDED order");
}

/*
 * KEY TEST (relocation path): libroot.so's OWN reference to `provider_id()` is
 * bound at load time (RTLD_NOW) by the loader's relocation resolver, which uses
 * the same DT_NEEDED-order search. root_provider_id() calls that bound
 * reference, so it must likewise return libbravo.so's id (2).
 */
static void test_relocation_binds_in_dt_needed_order(void *h)
{
    int (*root_provider_id)(void) = (int (*)(void))dlsym(h, "root_provider_id");
    if (!root_provider_id) {
        fail("relocation binds in DT_NEEDED order", dlerror());
        return;
    }

    int id = root_provider_id();
    if (id != BRAVO_ID) {
        char reason[128];
        snprintf(reason, sizeof(reason),
                 "root_provider_id()=%d (expected %d from libbravo.so, the first "
                 "DT_NEEDED entry); loader bound libroot.so's reference to "
                 "libalpha.so (%d)",
                 id, BRAVO_ID, ALPHA_ID);
        fail("relocation binds in DT_NEEDED order", reason);
        return;
    }
    pass("relocation binds provider_id() in DT_NEEDED order");
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn DT_NEEDED search-order tests ===\n");
    fflush(stdout);

    void *h = dlopen("lib/libroot.so", RTLD_NOW);
    if (h == NULL) {
        fail("dlopen(libroot.so)", dlerror());
        printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
        return 1;
    }

    test_both_dependencies_loaded(h);
    test_dlsym_resolves_in_dt_needed_order(h);
    test_relocation_binds_in_dt_needed_order(h);

    dlclose(h);

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
