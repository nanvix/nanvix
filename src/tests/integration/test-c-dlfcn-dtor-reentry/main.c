/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-dtor-reentry-c: Validate that a library stays discoverable, with its
 * dependency graph intact, until its `.fini_array` destructors have finished
 * running.
 *
 * Before the fix, dlclose() removed a library (and any dependency that became
 * unreferenced) from the loader registry, and detached its dependency edges,
 * *before* running the `.fini_array` destructors. A destructor that re-entered
 * the loader therefore observed a half-dismantled state: dlopen() of the
 * library being unloaded mapped a SECOND copy (new file descriptor -> new
 * handle) instead of returning the existing one, and dlsym() against the
 * library's own handle (or a dependency's symbol) failed because the entry /
 * edge was already gone.
 *
 * Fixtures (staged under lib/ by the per-suite RAMFS image):
 *   - libdep.so:     exports dep_value() == 4321. Self-contained.
 *   - libreentry.so: DT_NEEDED libdep.so; exports reentry_value() == 1234; its
 *                    `.fini_array` destructor calls dtor_probe() (below,
 *                    resolved from the main executable's exported global scope,
 *                    which the suite ELF publishes via PIE + --export-dynamic).
 *
 * Flow: main() dlopen()s libreentry.so, records the handle, then dlclose()s it.
 * dlclose() runs libreentry.so's destructor, which calls back into
 * dtor_probe(). While that destructor is on the stack -- i.e. WHILE the library
 * is being torn down -- dtor_probe() re-enters the loader and checks that:
 *   (1) dlopen("lib/libreentry.so") returns the SAME handle (no second copy);
 *   (2) dlsym(handle, "reentry_value") resolves the library's own symbol;
 *   (3) dlsym(handle, "dep_value") resolves a STILL-LOADED dependency's symbol;
 *   (4) dlsym(original_handle, "reentry_value") -- the handle main() still holds
 *       -- resolves too.
 *
 * Pass/fail is the guest exit code (the harness discards stdout in standalone
 * mode): main() returns 0 only when the destructor ran AND all four checks
 * passed.
 */

#include <dlfcn.h>
#include <stddef.h>
#include <unistd.h>

/* Sentinel proving the destructor (hence dtor_probe) actually ran. */
#define PROBE_SENTINEL 0x2538

/* Per-check result bits recorded by dtor_probe(). */
#define BIT_REOPEN_SAME_HANDLE 0x1 /* (1) dlopen returns the existing handle   */
#define BIT_SELF_SYM_OK        0x2 /* (2) dlsym resolves the library's symbol  */
#define BIT_DEP_SYM_OK         0x4 /* (3) dlsym resolves a dependency's symbol */
#define BIT_ORIG_SYM_OK        0x8 /* (4) the original handle still resolves   */
#define PROBE_EXPECTED \
    (BIT_REOPEN_SAME_HANDLE | BIT_SELF_SYM_OK | BIT_DEP_SYM_OK | BIT_ORIG_SYM_OK)

/*
 * Witnesses owned by the executable and exported via --export-dynamic so the
 * fixture's destructor can resolve `dtor_probe` from the loader's global scope.
 * `volatile` keeps the optimizer from assuming their values across the
 * cross-module write performed by the destructor.
 */
volatile int g_probe_ran = 0;
volatile int g_probe_result = 0;
volatile void *g_original_handle = NULL;

/*
 * Invoked from libreentry.so's `.fini_array` destructor, i.e. while dlclose()
 * is tearing libreentry.so down. Re-enters the loader to confirm the library
 * (and its dependency) are still discoverable during teardown, recording the
 * outcome in `g_probe_result` for main() to check after dlclose() returns.
 */
void dtor_probe(void)
{
    int result = 0;

    g_probe_ran = PROBE_SENTINEL;

    /*
     * (1) dlopen() of the library currently being unloaded must return the
     *     EXISTING registry entry (same handle), not map a second copy. A
     *     second copy would open a new file descriptor and hand back a
     *     different handle.
     */
    void *reopened = dlopen("lib/libreentry.so", RTLD_NOW);
    if (reopened != NULL) {
        if (reopened == (void *)g_original_handle) {
            result |= BIT_REOPEN_SAME_HANDLE;
        }

        /* (2) The library's own exported symbol resolves during teardown. */
        int (*rv)(void) = NULL;
        *(void **)(&rv) = dlsym(reopened, "reentry_value");
        if (rv != NULL && rv() == 1234) {
            result |= BIT_SELF_SYM_OK;
        }

        /*
         * (3) A symbol of a STILL-LOADED dependency resolves through the library's
         *     handle: libreentry.so -> libdep.so's edge must remain intact until
         *     the destructors finish.
         */
        int (*dv)(void) = NULL;
        *(void **)(&dv) = dlsym(reopened, "dep_value");
        if (dv != NULL && dv() == 4321) {
            result |= BIT_DEP_SYM_OK;
        }
    }

    /* (4) The original handle main() still holds also resolves symbols. */
    int (*rv2)(void) = NULL;
    *(void **)(&rv2) = dlsym((void *)g_original_handle, "reentry_value");
    if (rv2 != NULL && rv2() == 1234) {
        result |= BIT_ORIG_SYM_OK;
    }

    g_probe_result = result;
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* Load the fixture and remember its handle for the destructor-time probe. */
    void *handle = dlopen("lib/libreentry.so", RTLD_NOW);
    if (handle == NULL) {
        return 1;
    }
    g_original_handle = handle;

    /* Closing the library runs its destructor, which calls dtor_probe(). */
    if (dlclose(handle) != 0) {
        return 2;
    }

    /* The destructor must have run at all. */
    if (g_probe_ran != PROBE_SENTINEL) {
        return 3;
    }

    /* Every re-entrancy check must have passed. Encode the failing checks into
     * the exit code (bit set == check that did NOT pass) to aid post-mortem. */
    if (g_probe_result != PROBE_EXPECTED) {
        return 16 | (PROBE_EXPECTED & ~g_probe_result);
    }

    /* Success: emit the magic string some harnesses expect, then return 0. */
    const char *magic = "ok";
    write(STDOUT_FILENO, magic, 2);

    return 0;
}
