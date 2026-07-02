/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * libslowctor.so - fixture for dlfcn-init-concurrent-c.
 *
 * Carries a `.init_array` constructor whose execution is deliberately slow and
 * observable, so a second thread that dlopen()s this library while the
 * constructor is running can prove its dlopen() did NOT return until the
 * constructor finished.
 *
 * The constructor:
 *   1. calls ctor_mark_started() so the racing thread knows the constructor is
 *      running (and this library is therefore already resident in the loader
 *      registry, discoverable on the dedup fast path);
 *   2. busy-waits until ctor_racer_arrived() reports the racing thread is about
 *      to re-enter dlopen();
 *   3. busy-waits a further fixed window so that if the loader is buggy and lets
 *      the racing dlopen() return early, that early return is observable BEFORE
 *      the completion witnesses below are set;
 *   4. records SLOW_STATE_READY into g_slow_state (exported via slow_state())
 *      and calls ctor_mark_done().
 *
 * ctor_mark_started / ctor_racer_arrived / ctor_mark_done are defined in the
 * main executable and left UNDEFINED here; the loader resolves them from the
 * global scope at load time (the suite ELF is PIE + --export-dynamic).
 */

#include <stdatomic.h>

/* Value stored once the constructor has finished. Must match main.c. */
#define SLOW_STATE_READY 0x2476

/*
 * Iterations of the post-arrival window spin. Large enough that a buggy loader's
 * early return from the racing dlopen() (a few instructions) lands inside the
 * window, so the racing thread snapshots an unfinished constructor. `volatile`
 * on the loop counter keeps the -O2 build from eliminating the delay.
 */
#define SLOW_CTOR_WINDOW_SPINS 50000000L

/* Defined in the main executable, exported via --export-dynamic; resolved from
 * the loader's global symbol table at load time. */
extern void ctor_mark_started(void);
extern int ctor_racer_arrived(void);
extern void ctor_mark_done(void);

/* Side effect the constructor produces; observed via slow_state() by both the
 * loader thread and the racing thread. */
static atomic_int g_slow_state = ATOMIC_VAR_INIT(0);

/* Runs from `.init_array` before dlopen() returns to the loading thread. */
static void __attribute__((constructor)) slow_ctor(void)
{
    /* Announce that the constructor is running (and this library is resident). */
    ctor_mark_started();

    /* Busy-wait until the racing thread announces it is about to re-enter
     * dlopen() of this same library. */
    while (ctor_racer_arrived() == 0) {
        /* spin */
    }

    /* Widen the post-arrival window so a buggy loader's early return in the
     * racing thread is observable before the witnesses below are set. */
    for (volatile long i = 0; i < SLOW_CTOR_WINDOW_SPINS; i++) {
        /* spin */
    }

    atomic_store_explicit(&g_slow_state, SLOW_STATE_READY, memory_order_release);
    ctor_mark_done();
}

/* Exported so both threads can observe the constructor's side effect via
 * dlsym(). */
int slow_state(void)
{
    return atomic_load_explicit(&g_slow_state, memory_order_acquire);
}
