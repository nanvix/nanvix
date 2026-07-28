/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <poll.h>
#include <pthread.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

// Maximum length for file content.
#define POLL_TEST_DATA_MAX 256

// Timeout for poll() system call.
#define POLL_TIMEOUT 1000

// Short timeout used for timeout behavior checks.
#define POLL_SHORT_TIMEOUT 20

// Delay before a worker changes pipe state.
#define POLL_WORKER_DELAY 25

// Generous upper bound for a short timeout under the VM test scheduler.
#define POLL_SHORT_TIMEOUT_UPPER 500

// Event at POLL_WORKER_DELAY must wake comfortably before the one-second deadline.
#define POLL_EARLY_WAKE_LIMIT 750

// Pipe worker actions.
#define POLL_ACTION_WRITE 0
#define POLL_ACTION_CLOSE 1

//==================================================================================================
// Structures
//==================================================================================================

struct poll_pipe_action {
    int fd;
    int action;
};

struct poll_signal_worker {
    pid_t child;
    int stop_fd;
};

// Set by the SIGUSR1 handler used to interrupt poll().
static volatile sig_atomic_t poll_signal_caught;

//==================================================================================================
// Helper Functions
//==================================================================================================

// Returns elapsed milliseconds between two monotonic timestamps.
static long elapsed_milliseconds(const struct timespec *start, const struct timespec *end)
{
    long seconds = (long)(end->tv_sec - start->tv_sec);
    long nanoseconds = end->tv_nsec - start->tv_nsec;
    return seconds * 1000 + nanoseconds / 1000000;
}

// Changes pipe state after a short delay.
static void *run_pipe_action(void *arg)
{
    struct poll_pipe_action *action = (struct poll_pipe_action *)arg;

    const struct timespec delay = {
        .tv_sec = 0,
        .tv_nsec = POLL_WORKER_DELAY * 1000000,
    };
    assert(nanosleep(&delay, NULL) == 0);

    if (action->action == POLL_ACTION_WRITE) {
        const char byte = 'x';
        assert(write(action->fd, &byte, 1) == 1);
    } else {
        assert(action->action == POLL_ACTION_CLOSE);
        assert(close(action->fd) == 0);
    }
    return NULL;
}

// Records delivery of the signal used to interrupt poll().
static void handle_poll_signal(int signum)
{
    if (signum == SIGUSR1) {
        poll_signal_caught = 1;
    }
}

// Starts a child that repeatedly signals its parent until stopped.
static struct poll_signal_worker start_signal_worker(void)
{
    int stopfds[2];
    assert(pipe(stopfds) == 0);
    pid_t parent = getpid();
    pid_t child = fork();
    assert(child != -1);
    if (child == 0) {
        assert(close(stopfds[1]) == 0);
        int flags = fcntl(stopfds[0], F_GETFL);
        assert(flags != -1);
        assert(fcntl(stopfds[0], F_SETFL, flags | O_NONBLOCK) == 0);
        const struct timespec delay = {
            .tv_sec = 0,
            .tv_nsec = POLL_WORKER_DELAY * 1000000,
        };
        for (;;) {
            assert(nanosleep(&delay, NULL) == 0);
            char stop;
            ssize_t count = read(stopfds[0], &stop, 1);
            if (count == 1) {
                break;
            }
            assert(count == -1);
            assert(errno == EAGAIN);
            assert(kill(parent, SIGUSR1) == 0);
        }
        assert(close(stopfds[0]) == 0);
        _exit(0);
    }

    assert(close(stopfds[0]) == 0);
    return (struct poll_signal_worker){.child = child, .stop_fd = stopfds[1]};
}

// Stops and reaps a signal worker.
static void stop_signal_worker(struct poll_signal_worker worker)
{
    const char stop = 'x';
    ssize_t count;
    do {
        count = write(worker.stop_fd, &stop, 1);
    } while (count == -1 && errno == EINTR);
    assert(count == 1);
    assert(close(worker.stop_fd) == 0);

    int status;
    while (waitpid(worker.child, &status, 0) == -1) {
        assert(errno == EINTR);
    }
    assert(WIFEXITED(status));
    assert(WEXITSTATUS(status) == 0);
}

// Tests signal interruption and verifies that the following poll receives its own response.
static void test_poll_interruption(void)
{
    struct sigaction action;
    struct sigaction old_action;
    memset(&action, 0, sizeof(action));
    action.sa_handler = handle_poll_signal;
    assert(sigemptyset(&action.sa_mask) == 0);
    assert(sigaction(SIGUSR1, &action, &old_action) == 0);

    int pipefds[2];
    assert(pipe(pipefds) == 0);
    struct poll_signal_worker worker = start_signal_worker();

    struct pollfd read_end = {
        .fd = pipefds[0],
        .events = POLLIN,
        .revents = 0,
    };
    poll_signal_caught = 0;
    errno = 0;
    assert(poll(&read_end, 1, POLL_TIMEOUT) == -1);
    assert(errno == EINTR);
    assert(poll_signal_caught == 1);
    stop_signal_worker(worker);

    const char ready_byte = 'y';
    assert(write(pipefds[1], &ready_byte, 1) == 1);
    read_end.revents = 0;
    assert(poll(&read_end, 1, 0) == 1);
    assert(read_end.revents == POLLIN);

    assert(close(pipefds[0]) == 0);
    assert(close(pipefds[1]) == 0);
    assert(sigaction(SIGUSR1, &old_action, NULL) == 0);
}

// Tests empty sets, timeout behavior, ignored descriptors, and invalid descriptors.
static void test_poll_arguments(void)
{
    assert(poll(NULL, 0, 0) == 0);

    struct timespec start;
    struct timespec end;
    assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
    assert(poll(NULL, 0, POLL_SHORT_TIMEOUT) == 0);
    assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);
    assert(elapsed_milliseconds(&start, &end) >= POLL_SHORT_TIMEOUT);
    assert(elapsed_milliseconds(&start, &end) < POLL_SHORT_TIMEOUT_UPPER);

    errno = 0;
    assert(poll(NULL, OPEN_MAX + 1, 0) == -1);
    assert(errno == EINVAL);

    struct pollfd ignored = {
        .fd = -1,
        .events = POLLIN,
        .revents = (short)-1,
    };
    assert(poll(&ignored, 1, 0) == 0);
    assert(ignored.fd == -1);
    assert(ignored.events == POLLIN);
    assert(ignored.revents == 0);

    struct pollfd invalid = {
        .fd = OPEN_MAX,
        .events = 0,
        .revents = (short)-1,
    };
    assert(poll(&invalid, 1, 0) == 1);
    assert(invalid.events == 0);
    assert(invalid.revents == POLLNVAL);
}

// Tests regular-file readiness, duplicate entries, EOF, and O_NONBLOCK independence.
static void test_poll_regular_file(const char *filename)
{
    const char *data = "Hello Nanvix!";
    size_t data_len = strlen(data);
    char buffer[POLL_TEST_DATA_MAX + 1];

    assert(strlen(filename) <= NAME_MAX);
    assert(data_len <= POLL_TEST_DATA_MAX);

    int fd = open(filename, O_CREAT | O_EXCL | O_RDWR, S_IRUSR | S_IWUSR);
    assert(fd != -1);

    struct pollfd pfd = {
        .fd = fd,
        .events = POLLIN | POLLOUT | POLLRDBAND | POLLWRBAND,
        .revents = 0,
    };
    assert(poll(&pfd, 1, POLL_TIMEOUT) == 1);
    assert((pfd.revents & (POLLIN | POLLOUT)) == (POLLIN | POLLOUT));

    assert(write(fd, data, data_len) == (ssize_t)data_len);
    assert(lseek(fd, 0, SEEK_SET) == 0);
    assert(read(fd, buffer, data_len) == (ssize_t)data_len);
    buffer[data_len] = '\0';
    assert(strcmp(buffer, data) == 0);

    assert(lseek(fd, 0, SEEK_END) != -1);
    pfd.events = POLLIN;
    pfd.revents = 0;
    assert(poll(&pfd, 1, POLL_TIMEOUT) == 1);
    assert(pfd.revents == POLLIN);

    struct pollfd duplicates[2] = {
        {.fd = fd, .events = 0, .revents = (short)-1},
        {.fd = fd, .events = POLLIN, .revents = (short)-1},
    };
    assert(poll(duplicates, 2, 0) == 1);
    assert(duplicates[0].revents == 0);
    assert(duplicates[1].revents == POLLIN);

    int flags = fcntl(fd, F_GETFL);
    assert(flags != -1);
    assert(fcntl(fd, F_SETFL, flags | O_NONBLOCK) == 0);
    pfd.revents = 0;
    assert(poll(&pfd, 1, 0) == 1);
    assert(pfd.revents == POLLIN);

    pfd.events = POLLERR | POLLHUP | POLLNVAL;
    pfd.revents = (short)-1;
    assert(poll(&pfd, 1, 0) == 0);
    assert(pfd.revents == 0);

    assert(close(fd) == 0);
    assert(unlink(filename) == 0);
}

// Tests pipe readiness, early wakeup, hangup, errors, and an infinite wait.
static void test_poll_pipe(void)
{
    int pipefds[2];
    assert(pipe(pipefds) == 0);

    struct pollfd read_end = {
        .fd = pipefds[0],
        .events = POLLIN,
        .revents = (short)-1,
    };
    assert(poll(&read_end, 1, POLL_SHORT_TIMEOUT) == 0);
    assert(read_end.revents == 0);

    struct pollfd write_end = {
        .fd = pipefds[1],
        .events = POLLOUT,
        .revents = 0,
    };
    assert(poll(&write_end, 1, 0) == 1);
    assert(write_end.revents == POLLOUT);

    struct poll_pipe_action write_action = {
        .fd = pipefds[1],
        .action = POLL_ACTION_WRITE,
    };
    pthread_t writer;
    assert(pthread_create(&writer, NULL, run_pipe_action, &write_action) == 0);

    struct timespec start;
    struct timespec end;
    assert(clock_gettime(CLOCK_MONOTONIC, &start) == 0);
    assert(poll(&read_end, 1, POLL_TIMEOUT) == 1);
    assert(clock_gettime(CLOCK_MONOTONIC, &end) == 0);
    assert(read_end.revents == POLLIN);
    assert(elapsed_milliseconds(&start, &end) < POLL_EARLY_WAKE_LIMIT);
    assert(pthread_join(writer, NULL) == 0);

    char byte;
    assert(read(pipefds[0], &byte, 1) == 1);
    assert(byte == 'x');
    assert(close(pipefds[1]) == 0);

    read_end.revents = 0;
    assert(poll(&read_end, 1, 0) == 1);
    assert((read_end.revents & (POLLIN | POLLHUP)) == (POLLIN | POLLHUP));
    assert(close(pipefds[0]) == 0);

    assert(pipe(pipefds) == 0);
    assert(close(pipefds[0]) == 0);
    write_end.fd = pipefds[1];
    write_end.events = 0;
    write_end.revents = 0;
    assert(poll(&write_end, 1, 0) == 1);
    assert(write_end.revents == POLLERR);
    assert(close(pipefds[1]) == 0);

    assert(pipe(pipefds) == 0);
    struct poll_pipe_action close_action = {
        .fd = pipefds[1],
        .action = POLL_ACTION_CLOSE,
    };
    pthread_t closer;
    assert(pthread_create(&closer, NULL, run_pipe_action, &close_action) == 0);
    read_end.fd = pipefds[0];
    read_end.events = 0;
    read_end.revents = 0;
    assert(poll(&read_end, 1, -1) == 1);
    assert(read_end.revents == POLLHUP);
    assert(pthread_join(closer, NULL) == 0);
    assert(close(pipefds[0]) == 0);
}

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether we can poll a file descriptor for read/write readiness.
void test_poll(void)
{
    fprintf(stderr, "testing poll() ... ");

    test_poll_arguments();
    test_poll_regular_file("poll_testfile.tmp");
    test_poll_pipe();
    test_poll_interruption();

    fprintf(stderr, "passed\n");
}

// Tests whether we can poll a hostfsd-backed regular file.
void test_poll_hostfs(void)
{
    if (getenv("NANVIX_TEST_HOSTFS") == NULL) {
        return;
    }

    fprintf(stderr, "testing poll() with hostfs ... ");

    test_poll_regular_file("/mnt/poll_testfile.tmp");

    fprintf(stderr, "passed\n");
}
