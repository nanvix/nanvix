/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 *
 * dlfcn-dlclose-cycle-c: dlclose() over a multiply-referenced dependency graph.
 *
 *   libroot.so  DT_NEEDED libmidx.so, libmidy.so, libleaf.so  (direct edge)
 *   libmidx.so  DT_NEEDED libleaf.so
 *   libmidy.so  DT_NEEDED libleaf.so
 *   libleaf.so  (no dependencies)
 *
 * dlclose() tears a library's dependency graph down by walking its DT_NEEDED
 * edges. When a single library is reachable through more than one edge, that
 * walk reaches it more than once. The pre-fix loader removed each dependency
 * with `extract_if` and asserted that exactly one entry was removed per step;
 * the second time a node was reached the removal found nothing and the
 * `assert_eq!(len, 1)` panicked, crashing the whole process. The fix replaces
 * that with a reference-count peel that records visited nodes, so a repeated
 * visit is handled gracefully and each library is unloaded exactly once -- and
 * only after every edge that references it has been released.
 *
 * A true DT_NEEDED cycle (libA <-> libB) is refused at load time by dlopen(),
 * so it can never reach dlclose(). The graph above is the loadable equivalent
 * that still drives the repeated-visit path: libleaf.so is reachable from
 * libroot.so through three edges (via libmidx.so, via libmidy.so, and directly),
 * so a single dlclose(libroot.so) visits it three times.
 *
 * This suite asserts the fixed behavior:
 *   * a well-formed, dependency-free control library loads (proves the RAMFS is
 *     mounted and the loader resolves a library);
 *   * dlopen(libroot.so) binds all three libleaf.so edges to ONE shared
 *     instance, and dlclose(libroot.so) returns cleanly (no panic) and unloads
 *     the entire graph -- reproducing and pinning the regression fix;
 *   * with libmidx.so and libmidy.so opened independently, closing one leaves
 *     the shared libleaf.so loaded for the other, and only closing the last
 *     reference unloads it ("unloaded only when all references are released");
 *   * the loader stays usable afterwards.
 *
 * Reaching any assertion after a dlclose() at all is itself the witness that
 * dlclose() returned rather than panicking; a pre-fix loader would have aborted
 * the process inside the multiply-referenced teardown.
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
 * Opens libleaf.so by itself and confirms its counter reads as a freshly
 * initialized zero. Used both as the opening sanity check and, after a graph is
 * torn down, to prove libleaf.so was genuinely unloaded: a re-load re-creates
 * its zero-initialized counter, so a non-zero value would mean a stale instance
 * survived the dlclose(). Returns 1 on success, 0 on failure.
 */
static int leaf_reloads_fresh(const char *name)
{
    void *h = dlopen("lib/libleaf.so", RTLD_NOW);
    if (h == NULL) {
        fail(name, dlerror());
        return 0;
    }

    int (*get)(void) = (int (*)(void))dlsym(h, "leaf_get");
    if (get == NULL) {
        fail(name, "leaf_get() not found");
        dlclose(h);
        return 0;
    }
    if (get() != 0) {
        fail(name, "libleaf.so counter was not fresh -- a stale instance survived");
        dlclose(h);
        return 0;
    }

    if (dlclose(h) != 0) {
        fail(name, "dlclose(libleaf.so) failed");
        return 0;
    }
    return 1;
}

/*
 * Test 0 (positive control): the dependency-free libleaf.so loads on its own.
 */
static void test_positive_control(void)
{
    if (leaf_reloads_fresh("dlopen(libleaf.so) [positive control]")) {
        pass("dlopen(libleaf.so) [positive control]");
    }
}

/*
 * Test 1: dlopen(libroot.so) binds every libleaf.so edge to a single shared
 * instance, then dlclose(libroot.so) tears the whole graph down without
 * panicking on the multiply-referenced libleaf.so and unloads it exactly once.
 * This is the direct reproduction of the repeated-visit regression.
 */
static void test_multipath_dlclose(void)
{
    const char *name = "dlclose(libroot.so) [multiply-referenced leaf]";

    void *h = dlopen("lib/libroot.so", RTLD_NOW);
    if (h == NULL) {
        fail(name, dlerror());
        return;
    }

    int (*midx_touch)(void) = (int (*)(void))dlsym(h, "midx_touch");
    int (*midy_touch)(void) = (int (*)(void))dlsym(h, "midy_touch");
    int (*root_touch)(void) = (int (*)(void))dlsym(h, "root_touch");
    int (*leaf_get)(void) = (int (*)(void))dlsym(h, "leaf_get");
    if (!midx_touch || !midy_touch || !root_touch || !leaf_get) {
        fail(name, "missing symbol across the dependency graph");
        dlclose(h);
        return;
    }

    /*
     * midx and midy bump the SAME leaf counter: if libleaf.so had been loaded
     * twice, midy_touch() would restart from 1 instead of continuing to 2.
     */
    if (midx_touch() != 1 || midy_touch() != 2) {
        fail(name, "libmidx.so and libmidy.so do not share one libleaf.so");
        dlclose(h);
        return;
    }

    /*
     * root_touch() bumps the counter through all three of libroot.so's edges
     * (via midx, via midy, and directly): 2 -> 3 -> 4 -> 5. A value other than 5
     * would mean libroot.so's direct libleaf.so edge resolved to a different
     * instance than the intermediates use.
     */
    if (root_touch() != 5 || leaf_get() != 5) {
        fail(name, "libroot.so's direct libleaf.so edge is not the shared instance");
        dlclose(h);
        return;
    }

    /*
     * The teardown that tripped the regression: libleaf.so is reached three
     * times. A clean return (dlclose() == 0) is the witness that the peel did
     * not panic.
     */
    if (dlclose(h) != 0) {
        fail(name, "dlclose(libroot.so) failed");
        return;
    }

    /* The graph had no other references, so it must have fully unloaded. */
    if (!leaf_reloads_fresh(name)) {
        return;
    }

    pass(name);
}

/*
 * Test 2: libmidx.so and libmidy.so are opened independently and share one
 * libleaf.so. Closing libmidx.so must leave libleaf.so loaded (libmidy.so still
 * references it); only closing libmidy.so releases the final reference and
 * unloads libleaf.so. This pins the "unloaded only when all references are
 * released" half of the regression.
 */
static void test_shared_dependency_refcount(void)
{
    const char *name = "dlclose() unloads shared libleaf.so only on last reference";

    void *hx = dlopen("lib/libmidx.so", RTLD_NOW);
    if (hx == NULL) {
        fail(name, dlerror());
        return;
    }
    void *hy = dlopen("lib/libmidy.so", RTLD_NOW);
    if (hy == NULL) {
        fail(name, dlerror());
        dlclose(hx);
        return;
    }

    int (*midx_touch)(void) = (int (*)(void))dlsym(hx, "midx_touch");
    /*
     * Resolve libmidy.so's entry point (and the shared leaf accessor) up front,
     * so they can be called after libmidx.so is closed. The function pointers
     * stay valid because libmidy.so and libleaf.so remain mapped until hy is
     * closed.
     */
    int (*midy_touch)(void) = (int (*)(void))dlsym(hy, "midy_touch");
    int (*leaf_get)(void) = (int (*)(void))dlsym(hy, "leaf_get");
    if (!midx_touch || !midy_touch || !leaf_get) {
        fail(name, "missing symbol across the shared dependency");
        dlclose(hy);
        dlclose(hx);
        return;
    }

    /* Shared instance: 1 (via midx) then 2 (via midy). */
    if (midx_touch() != 1 || midy_touch() != 2) {
        fail(name, "libmidx.so and libmidy.so do not share one libleaf.so");
        dlclose(hy);
        dlclose(hx);
        return;
    }

    /* Close one referrer. libleaf.so must stay loaded for libmidy.so. */
    if (dlclose(hx) != 0) {
        fail(name, "dlclose(libmidx.so) failed");
        dlclose(hy);
        return;
    }

    /*
     * If closing libmidx.so had wrongly unloaded the shared libleaf.so, this
     * call would either fault or observe a counter reset to 1; a correct loader
     * continues the shared counter to 3.
     */
    if (midy_touch() != 3 || leaf_get() != 3) {
        fail(name, "shared libleaf.so was unloaded while libmidy.so still referenced it");
        dlclose(hy);
        return;
    }

    /* Release the last referrer: libleaf.so must now unload. */
    if (dlclose(hy) != 0) {
        fail(name, "dlclose(libmidy.so) failed");
        return;
    }
    if (!leaf_reloads_fresh(name)) {
        return;
    }

    pass(name);
}

/*
 * Test 3: after all of the above teardown, the loader is still usable -- the
 * whole graph loads and unloads cleanly one more time.
 */
static void test_loader_still_usable(void)
{
    const char *name = "loader still usable after multi-reference dlclose";

    void *h = dlopen("lib/libroot.so", RTLD_NOW);
    if (h == NULL) {
        fail(name, dlerror());
        return;
    }
    int (*root_touch)(void) = (int (*)(void))dlsym(h, "root_touch");
    if (!root_touch || root_touch() != 3) {
        fail(name, "libroot.so did not reload cleanly");
        dlclose(h);
        return;
    }
    if (dlclose(h) != 0) {
        fail(name, "dlclose(libroot.so) failed");
        return;
    }
    pass(name);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn dlclose multiply-referenced graph tests ===\n");
    fflush(stdout);

    test_positive_control();
    test_multipath_dlclose();
    test_shared_dependency_refcount();
    test_loader_still_usable();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
