/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdatomic.h>
#include <stdint.h>
#include <unistd.h>

#define TEST_TIMEOUT_SECONDS 10

static atomic_int stage;
static volatile sig_atomic_t delivered;
static volatile pthread_t recipient;

static void handle_signal(int signum)
{
    assert(signum == SIGUSR1);
    recipient = pthread_self();
    delivered++;
}

static void wait_for_stage(int expected)
{
    while (atomic_load(&stage) != expected) {
        assert(sched_yield() == 0);
    }
}

static void *blocked_recipient(void *arg)
{
    (void)arg;
    sigset_t blocked;
    sigset_t pending;
    sigset_t empty;
    assert(sigemptyset(&blocked) == 0);
    assert(sigaddset(&blocked, SIGUSR1) == 0);
    assert(pthread_sigmask(SIG_BLOCK, &blocked, NULL) == 0);
    atomic_store(&stage, 1);
    wait_for_stage(2);

    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGUSR1) == 1);
    assert(delivered == 0);
    assert(sigemptyset(&empty) == 0);
    errno = 0;
    assert(sigsuspend(&empty) == -1);
    assert(errno == EINTR);
    assert(delivered == 1);
    assert(recipient == pthread_self());
    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGUSR1) == 0);
    return (NULL);
}

static void test_directed_pending(void)
{
    delivered = 0;
    atomic_store(&stage, 0);
    pthread_t worker;
    assert(pthread_create(&worker, NULL, blocked_recipient, NULL) == 0);
    wait_for_stage(1);

    errno = EDOM;
    assert(pthread_kill(worker, 0) == 0);
    assert(errno == EDOM);
    assert(pthread_kill(worker, SIGUSR1) == 0);
    assert(errno == EDOM);
    assert(pthread_kill(worker, SIGUSR1) == 0);
    assert(errno == EDOM);
    assert(delivered == 0);

    // An unblocked sibling must neither consume nor report the target's pending signal.
    sigset_t blocked;
    sigset_t pending;
    assert(sigemptyset(&blocked) == 0);
    assert(sigaddset(&blocked, SIGUSR1) == 0);
    assert(pthread_sigmask(SIG_BLOCK, &blocked, NULL) == 0);
    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGUSR1) == 0);
    assert(pthread_sigmask(SIG_UNBLOCK, &blocked, NULL) == 0);
    assert(delivered == 0);

    atomic_store(&stage, 2);
    assert(pthread_join(worker, NULL) == 0);
    assert(delivered == 1);
    assert(recipient == worker);
    errno = EDOM;
    assert(pthread_kill(worker, 0) == ESRCH);
    assert(errno == EDOM);
}

int main(void)
{
    alarm(TEST_TIMEOUT_SECONDS);
    pthread_t self = pthread_self();
    errno = EDOM;
    assert(pthread_kill(self, -1) == EINVAL);
    assert(errno == EDOM);
    assert(pthread_kill(self, NSIG) == EINVAL);
    assert(errno == EDOM);
    assert(pthread_kill((pthread_t)UINT32_MAX, 0) == ESRCH);
    assert(errno == EDOM);
    assert(pthread_kill(PTHREAD_NULL, SIGUSR1) == ESRCH);
    assert(errno == EDOM);

    struct sigaction action = {0};
    action.sa_handler = handle_signal;
    assert(sigemptyset(&action.sa_mask) == 0);
    assert(sigaction(SIGUSR1, &action, NULL) == 0);
    errno = EDOM;
    assert(pthread_kill(self, 0) == 0);
    assert(errno == EDOM);
    assert(delivered == 0);
    assert(pthread_kill(self, SIGUSR1) == 0);
    assert(errno == EDOM);
    assert(delivered == 1);
    assert(recipient == self);

    test_directed_pending();
    alarm(0);
    return (0);
}
