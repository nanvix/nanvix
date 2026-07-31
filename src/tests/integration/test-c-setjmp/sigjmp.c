/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include "common.h"
#include <assert.h>
#include <setjmp.h>
#include <signal.h>
#include <stdio.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests sigsetjmp()/siglongjmp() non-local control flow.
void test_sigsetjmp_siglongjmp(void)
{
    fprintf(stderr, "testing sigsetjmp()/siglongjmp() ... ");

    sigjmp_buf env;
    volatile int reached = 0;

    switch (sigsetjmp(env, 1)) {
        case 0:
            reached = 1;
            siglongjmp(env, 5);
            assert(0 && "siglongjmp() returned");
        case 5:
            assert(reached == 1);
            break;
        default:
            assert(0 && "siglongjmp() delivered the wrong value");
    }

    fprintf(stderr, "passed\n");
}

// Tests that siglongjmp() restores the signal mask saved by sigsetjmp(env, 1).
void test_sigsetjmp_restores_signal_mask(void)
{
    fprintf(stderr, "testing sigsetjmp() signal-mask restoration ... ");

    sigjmp_buf env;
    sigset_t original;
    sigset_t saved;
    sigset_t changed;
    sigset_t current;
    volatile int reached = 0;

    assert(sigprocmask(SIG_SETMASK, NULL, &original) == 0);
    assert(sigemptyset(&saved) == 0);
    assert(sigaddset(&saved, SIGUSR1) == 0);
    assert(sigemptyset(&changed) == 0);
    assert(sigaddset(&changed, SIGUSR2) == 0);
    assert(sigprocmask(SIG_SETMASK, &saved, NULL) == 0);

    switch (sigsetjmp(env, 1)) {
        case 0:
            reached = 1;
            assert(sigprocmask(SIG_SETMASK, &changed, NULL) == 0);
            siglongjmp(env, 1);
            assert(0 && "siglongjmp() returned");
        case 1:
            assert(reached == 1);
            break;
        default:
            assert(0 && "siglongjmp() delivered the wrong value");
    }

    assert(sigprocmask(SIG_SETMASK, NULL, &current) == 0);
    assert(sigismember(&current, SIGUSR1) == 1);
    assert(sigismember(&current, SIGUSR2) == 0);
    assert(sigprocmask(SIG_SETMASK, &original, NULL) == 0);

    fprintf(stderr, "passed\n");
}

// Tests that sigsetjmp(env, 0) does not cause siglongjmp() to restore an earlier signal mask.
void test_sigsetjmp_without_savemask(void)
{
    fprintf(stderr, "testing sigsetjmp() without signal-mask saving ... ");

    sigjmp_buf env;
    sigset_t original;
    sigset_t initial;
    sigset_t changed;
    sigset_t current;
    volatile int reached = 0;

    assert(sigprocmask(SIG_SETMASK, NULL, &original) == 0);
    assert(sigemptyset(&initial) == 0);
    assert(sigaddset(&initial, SIGUSR1) == 0);
    assert(sigemptyset(&changed) == 0);
    assert(sigaddset(&changed, SIGUSR2) == 0);
    assert(sigprocmask(SIG_SETMASK, &initial, NULL) == 0);

    switch (sigsetjmp(env, 0)) {
        case 0:
            reached = 1;
            assert(sigprocmask(SIG_SETMASK, &changed, NULL) == 0);
            siglongjmp(env, 1);
            assert(0 && "siglongjmp() returned");
        case 1:
            assert(reached == 1);
            break;
        default:
            assert(0 && "siglongjmp() delivered the wrong value");
    }

    assert(sigprocmask(SIG_SETMASK, NULL, &current) == 0);
    assert(sigismember(&current, SIGUSR1) == 0);
    assert(sigismember(&current, SIGUSR2) == 1);
    assert(sigprocmask(SIG_SETMASK, &original, NULL) == 0);

    fprintf(stderr, "passed\n");
}
