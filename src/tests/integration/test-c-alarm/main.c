/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <limits.h>
#include <pthread.h>
#include <sched.h>
#include <signal.h>
#include <stdatomic.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define NANOSECONDS_PER_SECOND 1000000000LL

static volatile sig_atomic_t alarm_count;
static atomic_int worker_ready;
static atomic_int worker_done;

static void handle_alarm(int signum)
{
    (void)signum;
    alarm_count++;
}

static void delay_ms(long milliseconds)
{
    struct timespec delay = {
        .tv_sec = milliseconds / 1000,
        .tv_nsec = (milliseconds % 1000) * 1000000L,
    };
    while (nanosleep(&delay, &delay) != 0) {
        assert(errno == EINTR);
    }
}

static void test_replace_and_cancel(void)
{
    assert(alarm(0) == 0);
    assert(alarm(UINT_MAX) == 0);
    assert(alarm(UINT_MAX - 1) == UINT_MAX);
    assert(alarm(2) == UINT_MAX - 1);
    delay_ms(100);
    assert(alarm(0) == 2);
    assert(alarm(0) == 0);
    delay_ms(2100);
    assert(alarm_count == 0);
}

static void test_interrupt_sleep(void)
{
    struct timespec start;
    struct timespec end;
    struct timespec delay = {.tv_sec = 5};
    assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
    assert(alarm(1) == 0);
    errno = 0;
    assert(nanosleep(&delay, NULL) == -1);
    assert(errno == EINTR);
    assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);
    assert(alarm_count == 1);
    assert(alarm(0) == 0);
    long long elapsed = (long long)(end.tv_sec - start.tv_sec) * NANOSECONDS_PER_SECOND +
                        end.tv_nsec - start.tv_nsec;
    assert(elapsed >= NANOSECONDS_PER_SECOND);
    assert(elapsed < 3 * NANOSECONDS_PER_SECOND);
}

static void test_pending_alarm(void)
{
    sigset_t blocked;
    sigset_t old_mask;
    sigset_t pending;
    assert(sigemptyset(&blocked) == 0);
    assert(sigaddset(&blocked, SIGALRM) == 0);
    assert(sigprocmask(SIG_BLOCK, &blocked, &old_mask) == 0);
    assert(alarm(1) == 0);
    delay_ms(1200);
    assert(alarm_count == 1);
    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGALRM) == 1);
    assert(alarm(0) == 0);
    assert(sigprocmask(SIG_SETMASK, &old_mask, NULL) == 0);
    assert(alarm_count == 2);
    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGALRM) == 0);
}

static void test_fork_does_not_inherit_alarm(void)
{
    assert(alarm(5) == 0);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        _exit(alarm(0) == 0 ? 0 : 1);
    }
    unsigned int remaining = alarm(0);
    assert(remaining > 0 && remaining <= 5);
    int status = 0;
    assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status) && WEXITSTATUS(status) == 0);
}

static void test_ignored_alarm(void)
{
    struct sigaction ignored = {0};
    struct sigaction previous;
    ignored.sa_handler = SIG_IGN;
    assert(sigemptyset(&ignored.sa_mask) == 0);
    assert(sigaction(SIGALRM, &ignored, &previous) == 0);
    assert(alarm(1) == 0);
    delay_ms(1200);
    assert(alarm(0) == 0);
    assert(alarm_count == 2);
    sigset_t pending;
    assert(sigpending(&pending) == 0);
    assert(sigismember(&pending, SIGALRM) == 0);
    assert(sigaction(SIGALRM, &previous, NULL) == 0);
}

static void *suspend_pending_alarm(void *arg)
{
    (void)arg;
    sigset_t mask;
    assert(sigemptyset(&mask) == 0);
    sigsuspend(&mask);
    _exit(0);
}

static void test_blocked_default_alarm(int suspend_mode)
{
    int pipefd[2];
    assert(pipe(pipefd) == 0);
    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        assert(close(pipefd[0]) == 0);
        struct sigaction action = {0};
        action.sa_handler = SIG_DFL;
        assert(sigemptyset(&action.sa_mask) == 0);
        assert(sigaction(SIGALRM, &action, NULL) == 0);
        sigset_t blocked;
        assert(sigemptyset(&blocked) == 0);
        assert(sigaddset(&blocked, SIGALRM) == 0);
        assert(sigprocmask(SIG_BLOCK, &blocked, NULL) == 0);
        assert(alarm(1) == 0);
        delay_ms(1200);
        sigset_t pending;
        assert(sigpending(&pending) == 0);
        assert(sigismember(&pending, SIGALRM) == 1);
        const unsigned char marker = 0x5a;
        assert(write(pipefd[1], &marker, sizeof(marker)) == sizeof(marker));
        if (suspend_mode == 1) {
            suspend_pending_alarm(NULL);
        } else if (suspend_mode == 2) {
            pthread_t thread;
            assert(pthread_create(&thread, NULL, suspend_pending_alarm, NULL) == 0);
            // Keep a masked sibling runnable after the pending signal is unmasked by sigsuspend.
            for (;;) {
                assert(sched_yield() == 0);
            }
        }
        assert(sigprocmask(SIG_UNBLOCK, &blocked, NULL) == 0);
        _exit(0);
    }
    assert(close(pipefd[1]) == 0);
    unsigned char marker = 0;
    assert(read(pipefd[0], &marker, sizeof(marker)) == sizeof(marker));
    assert(marker == 0x5a);
    assert(close(pipefd[0]) == 0);
    int status = 0;
    pid_t reaped = 0;
    for (int retry = 0; retry < 100 && reaped == 0; retry++) {
        reaped = waitpid(child, &status, WNOHANG);
        assert(reaped >= 0);
        if (reaped == 0) {
            delay_ms(50);
        }
    }
    assert(reaped == child);
    // Nanvix reports the kernel's fatal-signal termination as an EINTR exit status.
    assert(WIFEXITED(status) && WEXITSTATUS(status) == EINTR);
}

static void *unmasked_sleeper(void *arg)
{
    (void)arg;
    sigset_t unblocked;
    assert(sigemptyset(&unblocked) == 0);
    assert(sigaddset(&unblocked, SIGALRM) == 0);
    assert(pthread_sigmask(SIG_UNBLOCK, &unblocked, NULL) == 0);
    atomic_store(&worker_ready, 1);
    struct timespec delay = {.tv_sec = 5};
    assert(nanosleep(&delay, NULL) == -1);
    assert(errno == EINTR);
    atomic_store(&worker_done, 1);
    return (NULL);
}

static void test_masked_runnable_sibling(void)
{
    sigset_t blocked;
    sigset_t old_mask;
    assert(sigemptyset(&blocked) == 0);
    assert(sigaddset(&blocked, SIGALRM) == 0);
    assert(pthread_sigmask(SIG_BLOCK, &blocked, &old_mask) == 0);
    pthread_t thread;
    assert(pthread_create(&thread, NULL, unmasked_sleeper, NULL) == 0);
    while (!atomic_load(&worker_ready)) {
        assert(sched_yield() == 0);
    }
    struct timespec start;
    assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
    assert(alarm(1) == 0);
    // Keep the process runnable while only its sleeping thread can accept SIGALRM.
    while (!atomic_load(&worker_done)) {
        struct timespec now;
        assert(clock_gettime(CLOCK_MONOTONIC, &now) == 0);
        assert(now.tv_sec - start.tv_sec < 5);
    }
    assert(pthread_join(thread, NULL) == 0);
    assert(alarm_count == 3);
    assert(alarm(0) == 0);
    assert(pthread_sigmask(SIG_SETMASK, &old_mask, NULL) == 0);
}

int main(void)
{
    struct sigaction action = {0};
    action.sa_handler = handle_alarm;
    assert(sigemptyset(&action.sa_mask) == 0);
    assert(sigaction(SIGALRM, &action, NULL) == 0);
    test_replace_and_cancel();
    test_interrupt_sleep();
    test_pending_alarm();
    test_fork_does_not_inherit_alarm();
    test_ignored_alarm();
    test_blocked_default_alarm(0);
    test_blocked_default_alarm(1);
    test_blocked_default_alarm(2);
    test_masked_runnable_sibling();
    return (0);
}
