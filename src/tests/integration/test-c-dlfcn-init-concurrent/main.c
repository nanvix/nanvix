/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * dlfcn-init-concurrent-c: Validate that a concurrent dlopen() of a library
 * does not observe it before its `.init_array` constructors have finished
 * running.
 *
 * dlopen() drops the loader registry lock before invoking a newly loaded
 * library's constructors, so that a constructor may legally call back into the
 * loader (dlsym, and a re-entrant dlopen of the same library). Before the fix,
 * that released window let a SECOND thread calling dlopen() on the same
 * filename hit the dedup fast path and get the handle back the instant the
 * registry lock was released -- well before the constructors had run. A dlsym()
 * through that handle could then reference state a constructor had not yet
 * initialized.
 *
 * Fixture (staged under lib/ by the per-suite RAMFS image):
 *   - libslowctor.so: a `.init_array` constructor that (1) announces it is
 *     running, (2) busy-waits until the racing thread says it is about to
 *     re-enter dlopen, (3) busy-waits a further fixed window so a buggy loader's
 *     early return is observable, then (4) records SLOW_STATE_READY into its own
 *     `slow_state()` and marks the run finished. The entry points it calls
 *     (ctor_mark_started / ctor_racer_arrived / ctor_mark_done) live in this
 *     executable and are resolved from the loader's global scope at load time
 *     (the suite ELF is PIE + --export-dynamic).
 *
 * Flow: main() spawns a racing thread and then dlopen()s libslowctor.so.
 * Because the racing thread is parked until the constructor announces itself,
 * main() deterministically wins the registry insert and is the thread that runs
 * the constructor. The racing thread then dlopen()s the SAME library while that
 * constructor is still on main()'s stack.
 *
 * Pass/fail is the guest exit code (the harness discards stdout in standalone
 * mode): main() returns 0 only when the racing dlopen() returned AFTER the
 * constructor finished AND a dlsym() through the racing thread's handle observes
 * the constructor's side effect. A buggy loader lets the racing dlopen() return
 * early, so the racing thread snapshots an unfinished constructor (exit 8) or a
 * pre-constructor slow_state() (exit 9).
 */

#include <dlfcn.h>
#include <pthread.h>
#include <stddef.h>
#include <stdatomic.h>
#include <unistd.h>

/* Value the constructor stores into libslowctor.so's slow_state() once it has
 * finished. Must match the definition in libs/slowctor.c. */
#define SLOW_STATE_READY 0x2476

/*
 * Witnesses owned by the executable. These are atomics because the constructor
 * and the racing thread intentionally communicate without taking a pthread lock.
 */
static atomic_int g_ctor_started = ATOMIC_VAR_INIT(0); /* set by the constructor */
static atomic_int g_racer_arrived = ATOMIC_VAR_INIT(0); /* set by the racer       */
static atomic_int g_ctor_done = ATOMIC_VAR_INIT(0);     /* set by the constructor */

/* Outcome recorded by the racing thread for main() to check after join. */
static void *g_racer_handle = NULL;
static int g_racer_saw_ctor_done = -1;
static int g_racer_slow_state = -1;

/* Entry points called from libslowctor.so's constructor (resolved from this
 * executable's exported global scope at load time). */
void ctor_mark_started(void)
{
    atomic_store_explicit(&g_ctor_started, 1, memory_order_release);
}

int ctor_racer_arrived(void)
{
    return atomic_load_explicit(&g_racer_arrived, memory_order_acquire);
}

void ctor_mark_done(void)
{
    atomic_store_explicit(&g_ctor_done, 1, memory_order_release);
}

/*
 * Racing thread: dlopen()s libslowctor.so WHILE main()'s thread is running its
 * constructor, then records whether that dlopen() returned only after the
 * constructor finished.
 */
static void *racer_thread(void *arg)
{
    (void)arg;

    /* Wait until main()'s thread is inside the constructor. */
    while (atomic_load_explicit(&g_ctor_started, memory_order_acquire) == 0) {
        /* Busy-wait; the preemptive scheduler runs the loader thread. */
    }

    /* Announce arrival (releasing the constructor's wait), then immediately
     * race into dlopen() of the SAME library. */
    atomic_store_explicit(&g_racer_arrived, 1, memory_order_release);
    void *h = dlopen("lib/libslowctor.so", RTLD_NOW);
    g_racer_handle = h;

    /* Snapshot the constructor-completion witness the instant dlopen() returns:
     * with the fix this is 1 (dlopen waited for the constructor); a buggy loader
     * returns early while the constructor is still in its window, so it is 0. */
    g_racer_saw_ctor_done = atomic_load_explicit(&g_ctor_done, memory_order_acquire);

    if (h != NULL) {
        int (*fn)(void) = NULL;
        *(void **)(&fn) = dlsym(h, "slow_state");
        g_racer_slow_state = (fn != NULL) ? fn() : -2;
    }

    return NULL;
}

int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    pthread_t tid;
    if (pthread_create(&tid, NULL, racer_thread, NULL) != 0) {
        return 1;
    }

    /* This thread wins the registry insert (the racer is parked until the
     * constructor sets g_ctor_started), so it is deterministically the loader
     * that runs libslowctor.so's constructor. */
    void *h1 = dlopen("lib/libslowctor.so", RTLD_NOW);
    if (h1 == NULL) {
        return 2;
    }

    if (pthread_join(tid, NULL) != 0) {
        return 3;
    }

    /* The constructor ran to completion on this (loader) thread. */
    if (atomic_load_explicit(&g_ctor_done, memory_order_acquire) != 1) {
        return 4;
    }

    /* The loader thread's own handle observes the constructor's side effect. */
    int (*fn1)(void) = NULL;
    *(void **)(&fn1) = dlsym(h1, "slow_state");
    if (fn1 == NULL || fn1() != SLOW_STATE_READY) {
        return 5;
    }

    /* The racer obtained a handle... */
    if (g_racer_handle == NULL) {
        return 6;
    }
    /* ...to the very same library (dedup, no second copy)... */
    if (g_racer_handle != (void *)h1) {
        return 7;
    }
    /* ...whose dlopen() returned only AFTER the constructor finished (the
     * racing dlopen() must not observe the library early)... */
    if (g_racer_saw_ctor_done != 1) {
        return 8;
    }
    /* ...and a dlsym() through the racer's handle observes the constructor's
     * side effect. */
    if (g_racer_slow_state != SLOW_STATE_READY) {
        return 9;
    }

    /* Success: emit the magic string some harnesses expect, then return 0. */
    const char *magic = "ok";
    write(STDOUT_FILENO, magic, 2);

    return 0;
}
