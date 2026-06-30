/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-initfini-c: Validate init/fini ordering across a startup-loaded
 * DT_NEEDED dependency.
 *
 * The executable is linked PIE + --export-dynamic against libinitfini.so
 * (-linitfini), so the static linker records a DT_NEEDED entry on it. NO
 * dlopen() is called: at process startup syscall::dlfcn::dllink_executable()
 * auto-loads libinitfini.so into the global scope and runs its `.init_array`
 * constructor BEFORE main(); at process exit __nanvix_libc_start_main() runs
 * its `.fini_array` destructor (via syscall::dlfcn::dlfini_executable()), the
 * reverse direction this suite exercises and that the other dlfcn suites do
 * not cover for the startup path.
 *
 * libinitfini.so is a "pure" side-effect dependency: the executable references
 * none of its symbols. Instead the fixture reaches back into this executable's
 * exported globals/helper (resolved from the loader's global scope, the same
 * mechanism as test-c-dlfcn-init-runpath's g_dtor_ran):
 *
 *   - its constructor writes CTOR_SENTINEL into `g_ctor_ran`, and
 *   - its destructor calls `test_dtor_finish()` (below) to decide the process
 *     exit code.
 *
 * Pass/fail is the guest exit code (the harness discards stdout in standalone
 * mode). The flow makes BOTH halves of the ordering observable through it:
 *
 *   1. main() fails fast with _exit(1) if the constructor did not run before
 *      it (g_ctor_ran unset).
 *   2. Otherwise main() records MAIN_SENTINEL in `g_main_ran` and returns a
 *      non-zero sentinel (EXIT_DTOR_DID_NOT_RUN). The fixture's destructor must
 *      OVERRIDE that sentinel with _exit(0) at exit. If the destructor never
 *      runs (init/fini ordering across the dependency not implemented), the
 *      sentinel propagates and the suite fails.
 *
 * So the suite returns 0 only when the constructor ran before main AND the
 * destructor ran at exit, in that order.
 */

#include <unistd.h>

/* Witness sentinels. Duplicated in libs/initfini.c — keep the two in sync. */
#define CTOR_SENTINEL 0xC70A
#define MAIN_SENTINEL 0x3A14

/* Exit codes propagated to the harness (only reached when the destructor did
 * NOT override the result with _exit(0)). */
#define EXIT_CTOR_DID_NOT_RUN 1
#define EXIT_DTOR_DID_NOT_RUN 42

/*
 * Witnesses owned by the executable and exported via --export-dynamic so the
 * DT_NEEDED fixture can resolve and update them from the loader's global scope.
 * `volatile` keeps the optimizer from assuming their values across the
 * cross-module writes performed by the fixture's constructor and by main().
 */
volatile int g_ctor_ran = 0;
volatile int g_main_ran = 0;

/*
 * Exit helper exported to the fixture's destructor. Centralizing the _exit()
 * here keeps the fixture from having to resolve libc's _exit from the global
 * scope: `test_dtor_finish` is defined in the executable's own translation
 * unit, so --export-dynamic is guaranteed to place it in .dynsym, and the
 * _exit it references is pulled into the executable.
 *
 * `ok` is non-zero when the destructor observed a correctly ordered
 * ctor -> main -> dtor sequence. _exit() makes the chosen code the authoritative
 * process exit status, overriding main()'s EXIT_DTOR_DID_NOT_RUN sentinel.
 */
void test_dtor_finish(int ok)
{
    _exit(ok ? 0 : 3);
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* The DT_NEEDED fixture's constructor must have run before main(). */
    if (g_ctor_ran != CTOR_SENTINEL) {
        _exit(EXIT_CTOR_DID_NOT_RUN);
    }

    /* Record that main() ran, so the destructor can confirm it runs AFTER. */
    g_main_ran = MAIN_SENTINEL;

    /*
     * Return a non-zero sentinel. The fixture's destructor must override it
     * with test_dtor_finish(1) -> _exit(0) at exit; if it never runs, this
     * sentinel is the process exit code and the suite fails.
     */
    return EXIT_DTOR_DID_NOT_RUN;
}
