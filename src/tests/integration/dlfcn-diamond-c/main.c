/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * dlfcn-diamond-c: Exercises diamond DT_NEEDED graph resolution.
 *
 *   libdiamond.so
 *     +-- DT_NEEDED libleft.so   --+
 *     +-- DT_NEEDED libright.so  --+-- DT_NEEDED libbase.so
 *     +-- DT_NEEDED libbase.so  ---/   (direct edge)
 *
 * libdiamond.so also carries a DIRECT DT_NEEDED on libbase.so (in
 * addition to the two arms that pull it in transitively). This shape
 * exercises both loader consolidation paths in one graph:
 *   * the two arms' libbase.so edges are consolidated by the registry
 *     scan run at each frame's entry (one arm loads libbase.so, the
 *     other binds to it);
 *   * libdiamond.so's own direct libbase.so edge is consolidated by the
 *     loader's load-loop re-check, because libbase.so is loaded by an
 *     arm AFTER libdiamond's entry scan already ran.
 *
 * A correct loader walks libdiamond.so's dependencies, opens libleft.so,
 * recurses into its DT_NEEDED list and opens libbase.so, then resumes
 * the outer loop for libright.so and its own direct libbase.so edge --
 * at which point libbase.so is already in the registry and must be
 * bound, NOT re-opened. A broken loader either:
 *   (a) deadlocks: while recursing into libleft.so it scans the registry
 *       and tries to lock libdiamond.so, whose mutex is still held by the
 *       outer recursive frame -- the guest hangs forever (the test
 *       harness catches this via its watchdog timeout), OR
 *   (b) re-opens libbase.so, ending up with two copies and two distinct
 *       `unique_counter` instances, OR
 *   (c) trips an internal `unreachable!()` / panics because the same
 *       file descriptor is now mapped to two registry entries.
 *
 * All three failure modes are caught by the assertions below (the
 * deadlock by the external watchdog, the duplication and panic by the
 * delta checks).
 */

#include <dlfcn.h>
#include <stdio.h>
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
 * Test 0 (control): dlopen libright.so directly. libright has a single
 * DT_NEEDED edge to libbase.so, so this exercises the depth-2 chain
 * without involving the diamond shape. If this hangs, the issue is
 * not specific to diamond resolution.
 */
static void test_chain_dlopen(void)
{
    void *h = dlopen("lib/libright.so", RTLD_NOW);
    if (h == NULL) {
        fail("dlopen(libright.so) [depth-2]", dlerror());
        return;
    }
    int (*rb)(void) = (int (*)(void))dlsym(h, "right_bump");
    if (!rb || rb() != 1) {
        fail("dlopen(libright.so) [depth-2]", "right_bump returned wrong value");
        dlclose(h);
        return;
    }
    dlclose(h);
    pass("dlopen(libright.so) [depth-2 chain]");
}

/*
 * Test 1: dlopen(libdiamond.so) must succeed (not hang, not error).
 * Successful return implies the loader walked the entire diamond
 * graph without re-opening libbase.so or otherwise dead-locking.
 */
static void test_diamond_dlopen(void)
{
    void *h = dlopen("lib/libdiamond.so", RTLD_NOW);
    if (h == NULL) {
        fail("dlopen(libdiamond.so)", dlerror());
        return;
    }

    int (*dl) (void) = (int (*)(void))dlsym(h, "diamond_left");
    int (*dr) (void) = (int (*)(void))dlsym(h, "diamond_right");
    int (*ob) (void) = (int (*)(void))dlsym(h, "diamond_observe");
    int (*db) (void) = (int (*)(void))dlsym(h, "diamond_base_get");
    if (!dl || !dr || !ob || !db) {
        fail("dlopen(libdiamond.so)", "missing diamond_* symbols");
        dlclose(h);
        return;
    }

    /*
     * Capture the baseline counter before the diamond arms run.  This
     * keeps the test robust against any prior `test_chain_dlopen()`
     * call leaving the counter at a non-zero value if Nanvix's
     * `dlclose` ever stops fully unmapping the library (POSIX permits
     * `dlclose` to defer the unmap).  We then assert deltas, not
     * absolute values:
     *
     *   left_val      = base_get() + 1 after left's base_bump()
     *   right_val     = base_get() + 2 after right's base_bump()
     *   observed      = base_get() + 2
     *   diamond_base  = base_get() + 2  (via libdiamond's direct edge)
     *
     * If libbase.so were loaded twice, libleft and libright would each
     * see their own private `unique_counter` starting at zero, so
     * left_val == 1, right_val == 1 (not 2), observed == 1 (not 2).
     */
    int (*base_get_addr)(void) = (int (*)(void))dlsym(h, "base_get");
    int baseline = base_get_addr ? base_get_addr() : 0;

    int left_val = dl();
    int right_val = dr();
    int observed = ob();
    int diamond_base = db();

    if (left_val != baseline + 1) {
        char reason[96];
        snprintf(reason, sizeof(reason),
                 "left_bump=%d, expected %d", left_val, baseline + 1);
        fail("dlopen(libdiamond.so)", reason);
        dlclose(h);
        return;
    }
    if (right_val != baseline + 2 || observed != baseline + 2) {
        /* This is the giveaway for a duplicate libbase.so: libleft and
         * libright each see their own private `unique_counter` instead
         * of sharing one. */
        char reason[160];
        snprintf(reason, sizeof(reason),
                 "right_bump=%d (expected %d), observed=%d (expected %d) "
                 "-- libbase.so was loaded twice",
                 right_val, baseline + 2, observed, baseline + 2);
        fail("dlopen(libdiamond.so)", reason);
        dlclose(h);
        return;
    }
    if (diamond_base != baseline + 2) {
        /* libdiamond's OWN direct DT_NEEDED edge on libbase.so resolved
         * to a different instance than the arms bumped -- the loader
         * re-opened libbase.so for the direct edge instead of binding to
         * the copy an arm already loaded (the load-loop re-check). */
        char reason[160];
        snprintf(reason, sizeof(reason),
                 "diamond_base_get=%d (expected %d) "
                 "-- libdiamond's direct libbase.so edge was not consolidated",
                 diamond_base, baseline + 2);
        fail("dlopen(libdiamond.so)", reason);
        dlclose(h);
        return;
    }

    dlclose(h);
    pass("dlopen(libdiamond.so)");
}

/*
 * Test 2: Second dlopen returns the same handle (libdiamond.so is
 * already in the registry).
 */
static void test_diamond_reopen(void)
{
    void *h1 = dlopen("lib/libdiamond.so", RTLD_NOW);
    if (h1 == NULL) {
        fail("re-dlopen(libdiamond.so)", dlerror());
        return;
    }

    void *h2 = dlopen("lib/libdiamond.so", RTLD_NOW);
    if (h2 == NULL) {
        fail("re-dlopen(libdiamond.so)", dlerror());
        dlclose(h1);
        return;
    }

    if (h1 != h2) {
        fail("re-dlopen(libdiamond.so)", "second dlopen returned a different handle");
        dlclose(h1);
        dlclose(h2);
        return;
    }

    dlclose(h1);
    dlclose(h2);
    pass("re-dlopen(libdiamond.so) returns same handle");
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn diamond DT_NEEDED tests ===\n");
    fflush(stdout);

    test_chain_dlopen();
    test_diamond_dlopen();
    test_diamond_reopen();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
