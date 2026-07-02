/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-ctor-dtor-reentry-c: Validate that a library's `.init_array` constructor
 * may dlopen() ANOTHER library, and its `.fini_array` destructor may dlsym() and
 * dlclose() that other library, without deadlocking on the loader's registry
 * lock.
 *
 * dlopen() drops the loader registry lock before running a newly loaded
 * library's constructors, and dlclose() runs destructors with no loader lock
 * held. That makes the loader re-entrant from constructor/destructor context: a
 * constructor may recursively dlopen() a further library, and a destructor may
 * recursively dlsym()/dlclose() one. Without that drop-and-resume structure
 * these paths would recursively acquire DYNAMIC_LIBRARY_REGISTRY and deadlock.
 *
 * Fixtures (staged under lib/ by the per-suite RAMFS image):
 *   - libother.so: exports other_value() == 0x2474; its `.fini_array` destructor
 *     calls other_report_dtor() (below) so the suite can confirm it was actually
 *     unloaded. This is the library opened by the constructor and closed by the
 *     destructor.
 *   - libhook.so:  its `.init_array` constructor calls hook_open_other() (which
 *     dlopen()s libother.so); its `.fini_array` destructor calls
 *     hook_close_other() (which dlsym()s then dlclose()s that handle). Both
 *     callbacks live in this executable and are resolved from the loader's global
 *     scope at load time (the suite ELF is PIE + --export-dynamic).
 *
 * Flow: main() dlopen()s libhook.so -- running its constructor, which opens
 * libother.so from inside the still-in-progress outer dlopen(). After the outer
 * dlopen() returns, main() confirms libother.so is visible to it. main() then
 * dlclose()s libhook.so, running its destructor, which uses and closes
 * libother.so from inside the in-progress dlclose().
 *
 * Pass/fail is the guest exit code (the harness discards stdout in standalone
 * mode): main() returns 0 only when both callbacks ran and every re-entrant
 * dlopen()/dlsym()/dlclose() succeeded. Non-zero codes encode where it failed
 * (see the returns below), with the 0x10/0x20 bands carrying the failing
 * per-check bits for post-mortem.
 */

#include <dlfcn.h>
#include <stddef.h>
#include <unistd.h>

/* Value libother.so's other_value() returns; must match libs/other.c. */
#define OTHER_VALUE 0x2474

/* Sentinels proving each callback actually ran (distinct, non-zero). */
#define CTOR_SENTINEL       0xC0C0
#define DTOR_SENTINEL       0xD0D0
#define OTHER_DTOR_SENTINEL 0xE0E0

/* Per-check result bits recorded by the constructor-side callback. */
#define BIT_CTOR_DLOPEN_OK 0x1 /* ctor's dlopen("libother.so") returned non-NULL */
#define BIT_CTOR_SYM_OK    0x2 /* ctor's dlsym(other, "other_value") resolved    */
#define CTOR_EXPECTED      (BIT_CTOR_DLOPEN_OK | BIT_CTOR_SYM_OK)

/* Per-check result bits recorded by the destructor-side callback. */
#define BIT_DTOR_SYM_OK     0x1 /* dtor's dlsym(other, "other_value") resolved */
#define BIT_DTOR_DLCLOSE_OK 0x2 /* dtor's dlclose(other) returned 0            */
#define DTOR_EXPECTED       (BIT_DTOR_SYM_OK | BIT_DTOR_DLCLOSE_OK)

/*
 * Witnesses owned by the executable. The constructor and destructor callbacks
 * (also in this executable) run on main()'s own thread -- the loader invokes
 * `.init_array`/`.fini_array` synchronously from dlopen()/dlclose() -- so plain
 * `volatile` (no atomics) is sufficient; there is no second thread. `volatile`
 * keeps the optimizer from caching these across the callbacks.
 */
static volatile int g_ctor_ran = 0;
static volatile int g_ctor_result = 0;
static volatile int g_dtor_ran = 0;
static volatile int g_dtor_result = 0;
static volatile int g_other_dtor_ran = 0;
static void *volatile g_other_handle = NULL;

/*
 * Called from libhook.so's constructor, i.e. from WITHIN main()'s dlopen() of
 * libhook.so. Recursively dlopen()s libother.so (the constructor-calls-dlopen
 * path) and resolves a symbol from it, recording the handle for the destructor.
 */
void hook_open_other(void)
{
    int result = 0;

    g_ctor_ran = CTOR_SENTINEL;

    void *other = dlopen("lib/libother.so", RTLD_NOW);
    g_other_handle = other;
    if (other != NULL) {
        result |= BIT_CTOR_DLOPEN_OK;

        int (*ov)(void) = NULL;
        *(void **)(&ov) = dlsym(other, "other_value");
        if (ov != NULL && ov() == OTHER_VALUE) {
            result |= BIT_CTOR_SYM_OK;
        }
    }

    g_ctor_result = result;
}

/*
 * Called from libhook.so's destructor, i.e. from WITHIN main()'s dlclose() of
 * libhook.so. Recursively dlsym()s (the destructor-calls-dlsym path) and then
 * dlclose()s libother.so (the destructor-calls-dlclose path). libother.so's own
 * destructor then sets g_other_dtor_ran, proving the recursive dlclose() ran to
 * completion.
 */
void hook_close_other(void)
{
    int result = 0;

    g_dtor_ran = DTOR_SENTINEL;

    if (g_other_handle != NULL) {
        int (*ov)(void) = NULL;
        *(void **)(&ov) = dlsym(g_other_handle, "other_value");
        if (ov != NULL && ov() == OTHER_VALUE) {
            result |= BIT_DTOR_SYM_OK;
        }

        if (dlclose(g_other_handle) == 0) {
            result |= BIT_DTOR_DLCLOSE_OK;
        }
    }

    g_dtor_result = result;
}

/*
 * Called from libother.so's destructor when the destructor-time dlclose() above
 * tears it down. Proves the recursive dlclose() actually unloaded libother.so.
 */
void other_report_dtor(void)
{
    g_other_dtor_ran = OTHER_DTOR_SENTINEL;
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    /* Load libhook.so; its constructor opens libother.so from inside this
     * dlopen() call. */
    void *hook = dlopen("lib/libhook.so", RTLD_NOW);
    if (hook == NULL) {
        return 1;
    }

    /* The constructor ran, and its recursive dlopen()/dlsym() both succeeded. */
    if (g_ctor_ran != CTOR_SENTINEL) {
        return 2;
    }
    if (g_ctor_result != CTOR_EXPECTED) {
        return 0x10 | (CTOR_EXPECTED & ~g_ctor_result);
    }
    if (g_other_handle == NULL) {
        return 3;
    }

    /* libother.so is visible to main() (the caller) after the outer dlopen()
     * returned: a dlopen() of it returns the SAME handle the constructor got
     * (dedup, no second copy), and its symbol resolves. The dedup open adds no
     * reference, so libother.so is left for the destructor to close. */
    void *other = dlopen("lib/libother.so", RTLD_NOW);
    if (other != g_other_handle) {
        return 4;
    }
    int (*ov)(void) = NULL;
    *(void **)(&ov) = dlsym(other, "other_value");
    if (ov == NULL || ov() != OTHER_VALUE) {
        return 5;
    }

    /* Close libhook.so; its destructor uses and closes libother.so from inside
     * this dlclose() call. */
    if (dlclose(hook) != 0) {
        return 6;
    }

    /* The destructor ran, and its recursive dlsym()/dlclose() both succeeded. */
    if (g_dtor_ran != DTOR_SENTINEL) {
        return 7;
    }
    if (g_dtor_result != DTOR_EXPECTED) {
        return 0x20 | (DTOR_EXPECTED & ~g_dtor_result);
    }

    /* The recursive dlclose() actually unloaded libother.so (its destructor
     * ran). */
    if (g_other_dtor_ran != OTHER_DTOR_SENTINEL) {
        return 8;
    }

    /* Success: emit the magic string some harnesses expect, then return 0. */
    const char *magic = "ok";
    write(STDOUT_FILENO, magic, 2);

    return 0;
}
