/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-searchpath-c: Validate that the loader's DEFAULT library search path
 * resolves a BARE shared-library name (no '/'), locating the REAL Nanvix shared
 * libraries libc.so and libm.so shipped under lib/.
 *
 * This is the acceptance test for the runtime layout + default search path work
 * (issue #2775). Unlike dlfcn-c (which dlopen()s an EXPLICIT relative path,
 * "lib/libmul.so", that bypasses the search path) and dlfcn-startup-c (which
 * links the executable against libc.so/libm.so via DT_NEEDED and lets the
 * crt0 startup loader auto-load them), this suite calls dlopen() at run time
 * with the BARE names "libc.so" and "libm.so". With no path separator,
 * syscall::dlfcn::resolve_library_path() must search the default
 * LIBRARY_SEARCH_PATHS ("lib/") and find lib/libc.so and lib/libm.so.
 * dlopen("libm.so") additionally exercises the default search path
 * transitively: libm.so carries a DT_NEEDED on libc.so (its allocator / mem*
 * surface), which the loader resolves through the same "lib/" search path and
 * consolidates onto the libc.so instance opened below.
 *
 * The UserVM has no shared rootfs, so each suite ships the libraries it needs in
 * its own per-suite RAMFS (handed to nanvixd via -ramfs), staged under lib/. The
 * loader finds them through the default lib/ search path exercised here.
 *
 * The suite is linked PIE + --export-dynamic (see the build wiring): libc.so
 * carries an UNDEFINED `__nanvix_main` (the app entry symbol, normally supplied
 * by the main executable), which the RTLD_NOW relocation of dlopen("libc.so")
 * resolves from this executable's exported global symbol scope.
 *
 * Only leaf / pure symbols are called from the freshly dlopen()ed copies
 * (strlen from libc.so, cos from libm.so): the executable is also statically
 * linked against libc.a, so calling a stateful symbol (e.g. malloc) from the
 * second in-memory libc would split the heap. Pass/fail is signalled by the
 * exit code (0 = pass), which nanvixd propagates; the "ok" write mirrors the
 * other suites' magic marker.
 */

#include <assert.h>
#include <dlfcn.h>
#include <string.h>
#include <unistd.h>

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /*
     * Bare name (no '/') -> resolve_library_path() searches the default
     * LIBRARY_SEARCH_PATHS ("lib/") and must find lib/libc.so. RTLD_GLOBAL keeps
     * libc's symbols in the global scope so libm.so's imports (loaded below)
     * bind to this instance.
     */
    void *libc = dlopen("libc.so", RTLD_NOW | RTLD_GLOBAL);
    assert(libc != NULL);

    /* strlen is a leaf libc symbol: safe to call from the second libc copy. */
    size_t (*p_strlen)(const char *) = NULL;
    *(void **)(&p_strlen) = dlsym(libc, "strlen");
    assert(p_strlen != NULL);
    assert(p_strlen("nanvix") == 6);

    /*
     * Bare name again -> the default search path finds lib/libm.so; libm.so's
     * DT_NEEDED libc.so is resolved through the same "lib/" search path and
     * consolidated onto the instance opened above.
     */
    void *libm = dlopen("libm.so", RTLD_NOW);
    assert(libm != NULL);

    /* cos is pure: cos(0.0) == 1.0 exactly, so no floating-point tolerance. */
    double (*p_cos)(double) = NULL;
    *(void **)(&p_cos) = dlsym(libm, "cos");
    assert(p_cos != NULL);
    assert(p_cos(0.0) == 1.0);

    assert(dlclose(libm) == 0);
    assert(dlclose(libc) == 0);

    /* Write magic string to signal that the test passed. */
    {
        const char *magic_string = "ok";
        write(STDOUT_FILENO, magic_string, 2);
    }

    return (0);
}
