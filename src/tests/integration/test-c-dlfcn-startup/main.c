/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-startup-c: Validate crt0's startup DT_NEEDED loader from BOTH angles it
 * is responsible for -- the eager GOT/PLT binding that lets startup-loaded
 * dependencies be called directly, AND the POSIX default-scope visibility that
 * lets those same dependencies be introspected through dlopen(NULL)/dlsym().
 *
 * Unlike dlfcn-selflink-c (which links the executable against a self-contained
 * libprovider.so fixture), this suite links against the actual shared libraries
 * shipped by the toolchain. The executable is linked PIE and against libm.so
 * (-l:libm.so) instead of the static libm.a, so the math symbols below are left
 * UNDEFINED in the executable and recorded as a DT_NEEDED entry plus
 * R_386_JMP_SLOT relocations. libm.so in turn imports libc.so's allocator and
 * mem* surface, so the executable also carries a DT_NEEDED on libc.so.
 *
 * At process startup, syscall::dlfcn::dllink_executable() walks the executable's
 * DT_NEEDED list and loads libc.so and then libm.so into the global scope (the
 * equivalent of dlopen(RTLD_GLOBAL | RTLD_NOW)), registering each dependency's
 * exported symbols in the loader's global symbol table and binding the
 * executable's GOT/PLT slots against them before .init_array / main(). libm.so's
 * own imports resolve from libc.so (loaded first); the executable's statically
 * linked libc.a remains the single heap owner, because the loader's global scope
 * is first-wins.
 *
 * Test 0 -- startup GOT/PLT binding. The math calls resolve
 * through the executable's own PLT, which the startup loader must have bound
 * before main(). No dlopen() is involved.
 *
 * Tests 1-3 -- dlopen(NULL) default scope includes startup DT_NEEDED libraries
 * from the default symbol scope. Per POSIX, dlopen(NULL) returns a handle whose
 * scope is the main executable PLUS every library loaded at startup; dlsym() on
 * that handle (and on the RTLD_DEFAULT pseudo-handle) must therefore find
 * symbols exported by libm.so. Before the fix, the global scope held only the
 * executable's own --export-dynamic symbols and libraries promoted with
 * RTLD_GLOBAL, so the startup DT_NEEDED dependencies were absent and these
 * lookups returned NULL.
 *
 * The math symbols are the clean probe for this: they are UNDEFINED in the
 * executable (only imported through the PLT) and dlinit() skips undefined
 * symbols, so they can never enter the global scope from the executable itself.
 * Resolving cos/pow/exp through dlopen(NULL) therefore proves that libm.so -- a
 * startup DT_NEEDED dependency -- is genuinely part of the default scope.
 *
 * Pass/fail is signalled by the exit code (0 = pass), which nanvixd propagates;
 * the trailing "ok" write mirrors the other suites' magic marker.
 */

#include <dlfcn.h>
#include <math.h>
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
    /* dlerror() may return NULL (no diagnostic available); guard against
     * passing a NULL pointer to printf("%s"), which is undefined behavior. */
    printf("  FAIL: %s (%s)\n", name, reason != NULL ? reason : "no diagnostic available");
    fflush(stdout);
    tests_failed++;
}

/* Function-pointer shapes of the libm.so symbols probed below. */
typedef double (*unary_fn)(double);
typedef double (*binary_fn)(double, double);

/*
 * Test 0: the startup loader bound the executable's GOT/PLT against libm.so
 * before main(), so calling the math symbols directly (through the PLT) yields
 * the correct results with NO dlopen() call.
 *
 * `volatile` inputs stop the optimizer (release builds compile at -O3) from
 * constant-folding these calls away: they must survive as real PLT references
 * into libm.so so the startup loader has GOT/PLT slots to bind. The chosen
 * arguments yield exact results, so the equality checks need no tolerance.
 */
static void test_startup_gotplt_binding(void)
{
    const char *name = "startup loader binds executable GOT/PLT before main()";

    volatile double zero = 0.0;
    volatile double base = 2.0;
    volatile double exponent = 10.0;

    double c = cos(zero);           /* cos(0)     == 1    */
    double p = pow(base, exponent); /* pow(2, 10) == 1024 */
    double e = exp(zero);           /* exp(0)     == 1    */

    if (c != 1.0 || p != 1024.0 || e != 1.0) {
        fail(name, "direct libm.so calls did not resolve at startup");
        return;
    }

    pass(name);
}

/*
 * Test 1: dlopen(NULL) returns a usable handle to the default symbol scope.
 * This is the entry point of the whole default-scope contract; a NULL return
 * would make every lookup below moot.
 */
static void test_dlopen_null_handle(void)
{
    const char *name = "dlopen(NULL) returns the default-scope handle";

    void *h = dlopen(NULL, RTLD_LAZY);
    if (h == NULL) {
        fail(name, dlerror());
        return;
    }

    /* Closing the default-scope handle must not tear anything down. */
    if (dlclose(h) != 0) {
        fail(name, "dlclose() of the dlopen(NULL) handle failed");
        return;
    }

    pass(name);
}

/*
 * Test 2: dlsym() on the dlopen(NULL) handle resolves symbols exported by the
 * startup-loaded libm.so, and the resolved pointers are the genuine
 * implementations (verified by calling them). Because the math symbols are
 * undefined in the executable, a non-NULL result here can ONLY come from libm.so
 * being present in the default scope.
 */
static void test_dlopen_null_resolves_startup_lib(void)
{
    const char *name = "dlsym(dlopen(NULL), ...) resolves startup DT_NEEDED libm.so";

    void *h = dlopen(NULL, RTLD_LAZY);
    if (h == NULL) {
        fail(name, dlerror());
        return;
    }

    unary_fn cos_p = (unary_fn)dlsym(h, "cos");
    binary_fn pow_p = (binary_fn)dlsym(h, "pow");
    unary_fn exp_p = (unary_fn)dlsym(h, "exp");
    if (cos_p == NULL || pow_p == NULL || exp_p == NULL) {
        fail(name, "libm.so symbols not visible in the dlopen(NULL) scope");
        dlclose(h);
        return;
    }

    /* Confirm the resolved addresses are the real functions, not stubs. */
    volatile double zero = 0.0;
    volatile double base = 2.0;
    volatile double exponent = 10.0;
    if (cos_p(zero) != 1.0 || pow_p(base, exponent) != 1024.0 || exp_p(zero) != 1.0) {
        fail(name, "symbols resolved through dlopen(NULL) called incorrectly");
        dlclose(h);
        return;
    }

    if (dlclose(h) != 0) {
        fail(name, "dlclose() of the dlopen(NULL) handle failed");
        return;
    }

    pass(name);
}

/*
 * Test 3: the RTLD_DEFAULT pseudo-handle (a NULL handle passed to dlsym)
 * resolves against the same default scope, so it too must see the startup-loaded
 * libm.so. A bogus name must still fail, proving the lookup is a genuine search
 * of the scope rather than an unconditional non-NULL return.
 */
static void test_rtld_default_scope(void)
{
    const char *name = "dlsym(RTLD_DEFAULT, ...) searches the default scope";

    unary_fn cos_p = (unary_fn)dlsym(RTLD_DEFAULT, "cos");
    if (cos_p == NULL) {
        fail(name, "libm.so symbol not visible via RTLD_DEFAULT");
        return;
    }

    volatile double zero = 0.0;
    if (cos_p(zero) != 1.0) {
        fail(name, "symbol resolved via RTLD_DEFAULT called incorrectly");
        return;
    }

    /* Clear any stale error state so the missing-symbol probe below observes
     * only the diagnostic produced by its own failed lookup. */
    (void)dlerror();
    if (dlsym(RTLD_DEFAULT, "nanvix_dlfcn_absent_symbol") != NULL) {
        fail(name, "RTLD_DEFAULT resolved a symbol that does not exist");
        return;
    }

    /* A genuine scope search must record a diagnostic on failure; a stub that
     * unconditionally returns NULL would leave dlerror() unset. */
    if (dlerror() == NULL) {
        fail(name, "RTLD_DEFAULT missing-symbol lookup did not set dlerror()");
        return;
    }

    pass(name);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    printf("=== dlfcn startup DT_NEEDED default-scope tests ===\n");
    fflush(stdout);

    test_startup_gotplt_binding();
    test_dlopen_null_handle();
    test_dlopen_null_resolves_startup_lib();
    test_rtld_default_scope();

    printf("\n%d passed, %d failed\n", tests_passed, tests_failed);
    fflush(stdout);

    if (tests_failed == 0) {
        const char *magic = "ok";
        write(STDOUT_FILENO, magic, 2);
    }

    return tests_failed > 0 ? 1 : 0;
}
