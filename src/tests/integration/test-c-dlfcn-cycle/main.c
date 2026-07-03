/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * dlfcn-cycle-c: DT_NEEDED cycle rejection.
 *
 *   libcyclea.so DT_NEEDED libcycleb.so
 *   libcycleb.so DT_NEEDED libcyclea.so   (two-node cycle)
 *
 *   libselfcycle.so DT_NEEDED libselfcycle.so   (single-node self-loop)
 *
 * The dynamic loader walks DT_NEEDED edges recursively when a library is
 * dlopen()ed. A cyclic edge must be detected: the frame that would re-open
 * an ancestor (or the library itself) has to bail out instead of recursing.
 * A loader that misses the cycle hands out a fresh file descriptor -- hence
 * a distinct handle -- for every recursive re-open of the ancestor, slips
 * past the already-loaded lookup, and recurses without bound until the
 * guest stack overflows. That failure mode is silent (an OOM / fault, not a
 * clean error), so this suite pins the fixed behavior: dlopen() of a cyclic
 * graph returns NULL cleanly, with dlerror() set, and every entry the failed
 * call added is rolled back so the loader stays usable afterwards.
 *
 * A correct loader:
 *   * loads the dependency-free control library libok.so (proves the RAMFS
 *     is mounted and a well-formed library resolves), then
 *   * refuses dlopen("lib/libcyclea.so") and dlopen("lib/libcycleb.so") with
 *     a NULL handle + non-NULL dlerror() (the two-node cycle), then
 *   * still refuses a repeat dlopen of the cyclic library deterministically
 *     and still loads libok.so afterwards (rollback left the registry
 *     clean), then
 *   * refuses dlopen("lib/libselfcycle.so") the same way (the self-loop).
 *
 * A broken (pre-fix) loader instead recurses without bound on the cyclic
 * libraries; the guest never returns from dlopen() and the test harness
 * catches the runaway via its external watchdog timeout. Every assertion
 * that a cyclic dlopen() returns NULL therefore also guards against the
 * unbounded-recursion regression, because reaching the assertion at all
 * requires dlopen() to have returned.
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
 * Loads the dependency-free control library and confirms one of its symbols
 * returns the expected sentinel. Returns 1 on success, 0 on failure. Used
 * both as the opening sanity check and to prove the loader still works after
 * the cyclic loads have been rejected and rolled back.
 */
static int load_control(const char *name)
{
    void *h = dlopen("lib/libok.so", RTLD_NOW);
    if (h == NULL) {
        fail(name, dlerror());
        return 0;
    }

    int (*ov)(void) = (int (*)(void))dlsym(h, "ok_value");
    if (!ov || ov() != 0x2479) {
        fail(name, "ok_value() missing or returned the wrong value");
        dlclose(h);
        return 0;
    }

    dlclose(h);
    return 1;
}

/*
 * Asserts that dlopen(path, RTLD_NOW) is refused cleanly: a NULL handle plus
 * a non-NULL dlerror() message. Calling dlerror() also clears the error state
 * for the next probe. Returns 1 on the expected rejection, 0 otherwise.
 *
 * Reaching this function's return at all means dlopen() came back rather than
 * recursing forever, so a `1` result is the positive witness for cycle
 * detection; the pre-fix loader would hang here instead and trip the
 * harness watchdog.
 */
static int expect_cycle_rejected(const char *path, const char *name)
{
    void *h = dlopen(path, RTLD_NOW);
    if (h != NULL) {
        fail(name, "expected NULL handle for a cyclic library, but it loaded");
        dlclose(h);
        return 0;
    }

    const char *error = dlerror();
    if (error == NULL) {
        fail(name, "dlopen() returned NULL without setting dlerror()");
        return 0;
    }

    return 1;
}

/*
 * Test 0 (positive control): a well-formed, dependency-free library loads.
 */
static void test_positive_control(void)
{
    if (load_control("dlopen(libok.so) [positive control]")) {
        pass("dlopen(libok.so) [positive control]");
    }
}

/*
 * Test 1: the two-node cycle libcyclea.so <-> libcycleb.so is rejected from
 * either entry point. Whichever library is opened first becomes the in-progress
 * ancestor; the recursion back onto it through the other node must be caught.
 */
static void test_two_node_cycle(void)
{
    if (!expect_cycle_rejected("lib/libcyclea.so", "dlopen(libcyclea.so) [cycle]")) {
        return;
    }
    if (!expect_cycle_rejected("lib/libcycleb.so", "dlopen(libcycleb.so) [cycle]")) {
        return;
    }
    pass("two-node DT_NEEDED cycle rejected from both entry points");
}

/*
 * Test 2: rejecting a cycle must roll back every entry the failed dlopen()
 * added, so a repeat attempt fails identically (not, say, by finding a
 * half-loaded stale copy) and an unrelated well-formed library still loads.
 */
static void test_cycle_rollback(void)
{
    if (!expect_cycle_rejected("lib/libcyclea.so", "re-dlopen(libcyclea.so) [rollback]")) {
        return;
    }
    if (!load_control("dlopen(libok.so) after rejected cycle [rollback]")) {
        return;
    }
    pass("cycle rejection rolled back cleanly (loader still usable)");
}

/*
 * Test 3: a single-node self-loop (libselfcycle.so DT_NEEDED libselfcycle.so)
 * is rejected the same way. This exercises the loader's direct self-cycle
 * branch rather than the multi-node ancestor branch above.
 */
static void test_self_cycle(void)
{
    if (expect_cycle_rejected("lib/libselfcycle.so", "dlopen(libselfcycle.so) [self-loop]")) {
        pass("single-node DT_NEEDED self-loop rejected");
    }
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn DT_NEEDED cycle tests ===\n");
    fflush(stdout);

    test_positive_control();
    test_two_node_cycle();
    test_cycle_rollback();
    test_self_cycle();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
