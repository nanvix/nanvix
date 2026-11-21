/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <string.h>
#include <time.h>
#include <unistd.h>

#define PAGE_SIZE 4096
#define ALARM_SECONDS 1
#define RETURN_BOUND_SECONDS 5
#define FILL_BYTE 0x31
#define WRITE_BYTE 0xa7
#define PROBE_BYTE 0xb8

static _Alignas(PAGE_SIZE) unsigned char buffer[2 * PAGE_SIZE];
static _Alignas(PAGE_SIZE) unsigned char filler[PAGE_SIZE];
static volatile sig_atomic_t alarm_count;

static void handle_alarm(int signum)
{
    int saved_errno = errno;
    (void)signum;
    alarm_count++;
    if (alarm_count > 1) {
        _exit(90);
    }
    // A second alarm fails the test rather than leaving a lost interruption blocked forever.
    alarm(RETURN_BOUND_SECONDS);
    errno = saved_errno;
}

static void start_alarm(struct timespec *start)
{
    alarm_count = 0;
    assert(clock_gettime(CLOCK_MONOTONIC, start) == 0);
    alarm(ALARM_SECONDS);
}

static void check_alarm(const struct timespec *start)
{
    struct timespec end;
    alarm(0);
    assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);
    assert(alarm_count == 1);
    long long elapsed_ns = (long long)(end.tv_sec - start->tv_sec) * 1000000000LL +
                           end.tv_nsec - start->tv_nsec;
    assert(elapsed_ns < (long long)RETURN_BOUND_SECONDS * 1000000000LL);
}

static size_t fill_pipe(int fd)
{
    int flags = fcntl(fd, F_GETFL);
    assert(flags >= 0);
    assert(fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0);
    size_t filled = 0;
    for (;;) {
        ssize_t count = write(fd, filler, sizeof(filler));
        if (count < 0) {
            assert(errno == EAGAIN);
            break;
        }
        assert(count == sizeof(filler));
        filled += (size_t)count;
    }
    assert(filled >= sizeof(buffer));
    assert(fcntl(fd, F_SETFL, flags) == 0);
    return (filled);
}

static void check_probe(const int pipefd[2])
{
    const unsigned char expected = PROBE_BYTE;
    unsigned char actual = 0;
    assert(write(pipefd[1], &expected, sizeof(expected)) == sizeof(expected));
    assert(read(pipefd[0], &actual, sizeof(actual)) == sizeof(actual));
    assert(actual == expected);
    assert(close(pipefd[0]) == 0);
    assert(close(pipefd[1]) == 0);
}

static void test_read(size_t available)
{
    int pipefd[2];
    assert(pipe(pipefd) == 0);
    if (available != 0) {
        assert(write(pipefd[1], filler, available) == (ssize_t)available);
    }
    memset(buffer, 0, sizeof(buffer));

    struct timespec start;
    start_alarm(&start);
    errno = 0;
    ssize_t count = read(pipefd[0], buffer, sizeof(buffer));
    int read_errno = errno;
    check_alarm(&start);
    if (available == 0) {
        assert(count == -1);
        assert(read_errno == EINTR);
    } else {
        assert(count == (ssize_t)available);
        assert(memcmp(buffer, filler, available) == 0);
        assert(buffer[available] == 0);
    }

    // A cancelled reader must not steal this next transfer or strand a response.
    check_probe(pipefd);
}

static void test_write(size_t room)
{
    int pipefd[2];
    assert(pipe(pipefd) == 0);
    size_t filled = fill_pipe(pipefd[1]);
    if (room != 0) {
        assert(read(pipefd[0], buffer, room) == (ssize_t)room);
    }
    memset(buffer, WRITE_BYTE, sizeof(buffer));

    struct timespec start;
    start_alarm(&start);
    errno = 0;
    ssize_t count = write(pipefd[1], buffer, sizeof(buffer));
    int write_errno = errno;
    check_alarm(&start);
    if (room == 0) {
        assert(count == -1);
        assert(write_errno == EINTR);
    } else {
        assert(count == (ssize_t)room);
    }

    // Drain every accepted byte and check that freeing space does not revive the cancelled tail.
    int flags = fcntl(pipefd[0], F_GETFL);
    assert(flags >= 0);
    assert(fcntl(pipefd[0], F_SETFL, flags | O_NONBLOCK) == 0);
    size_t drained = 0;
    for (;;) {
        count = read(pipefd[0], buffer, sizeof(buffer));
        if (count < 0) {
            assert(errno == EAGAIN);
            break;
        }
        assert(count > 0);
        assert(drained + (size_t)count <= filled);
        for (ssize_t i = 0; i < count; i++) {
            unsigned char expected = drained < filled - room ? FILL_BYTE : WRITE_BYTE;
            assert(buffer[i] == expected);
            drained++;
        }
    }
    assert(drained == filled);
    assert(fcntl(pipefd[0], F_SETFL, flags) == 0);
    check_probe(pipefd);
}

int main(void)
{
    struct sigaction action = {0};
    action.sa_handler = handle_alarm;
    assert(sigemptyset(&action.sa_mask) == 0);
    assert(sigaction(SIGALRM, &action, NULL) == 0);
    memset(filler, FILL_BYTE, sizeof(filler));

    test_read(0);
    test_write(0);
    test_write(PAGE_SIZE);
    test_read(PAGE_SIZE);
    return (0);
}
