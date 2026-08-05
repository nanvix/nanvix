/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Regression test for cancelling pipe operations parked in vfsd when SIGCHLD interrupts the
 * caller. The read scenario detects a stale reader stealing the next pull. The write scenario
 * releases buffer space only after cancellation returns, then checks that the cancelled byte was
 * not written. The final fork detects a response left in the caller's mailbox.
 */

#include <errno.h>
#include <fcntl.h>
#include <signal.h>
#include <string.h>
#include <sys/wait.h>
#include <time.h>
#include <unistd.h>

#define PIPE_FILL_CHUNK_SIZE 4096
#define PIPE_FILL_BYTE 0x31
#define CANCELLED_WRITE_BYTE 0xa7
#define COMMITTED_WRITE_BYTE 0xb8
#define SIGNAL_RPC_BYTE 0xd4

#define CHECK(step, condition) \
    do {                       \
        if (!(condition)) {    \
            return (step);     \
        }                      \
    } while (0)

static volatile sig_atomic_t sigchld_count;
static volatile sig_atomic_t signal_rpc_status;
static int signal_rpc_fd = -1;

static void handle_sigchld(int signum)
{
    int saved_errno = errno;
    const unsigned char value = SIGNAL_RPC_BYTE;

    (void)signum;
    // The interrupted outer RPC keeps a request active, so this nested RPC allocates the response
    // stash before issuing the write.
    signal_rpc_status =
        (write(signal_rpc_fd, &value, sizeof(value)) == sizeof(value)) ? 1 : -1;
    sigchld_count++;
    errno = saved_errno;
}

static int delay_ms(long milliseconds)
{
    struct timespec delay = {
        .tv_sec = milliseconds / 1000,
        .tv_nsec = (milliseconds % 1000) * 1000000L,
    };

    while (nanosleep(&delay, &delay) != 0) {
        if (errno != EINTR) {
            return (-1);
        }
    }
    return (0);
}

static int wait_success(pid_t pid)
{
    int status = 0;
    pid_t result;

    do {
        result = waitpid(pid, &status, 0);
    } while (result < 0 && errno == EINTR);

    return (result == pid && WIFEXITED(status) && WEXITSTATUS(status) == 0);
}

static ssize_t read_retry(int fd, void *buffer, size_t count, int *interrupted)
{
    for (;;) {
        ssize_t result = read(fd, buffer, count);
        if (result >= 0 || errno != EINTR) {
            return (result);
        }
        if (interrupted != NULL) {
            *interrupted = 1;
        }
    }
}

static int fill_pipe(int fd, size_t *filled)
{
    int flags = fcntl(fd, F_GETFL);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) != 0) {
        return (-1);
    }

    unsigned char buffer[PIPE_FILL_CHUNK_SIZE];
    memset(buffer, PIPE_FILL_BYTE, sizeof(buffer));
    size_t chunk_size = sizeof(buffer);
    *filled = 0;

    for (;;) {
        ssize_t count = write(fd, buffer, chunk_size);
        if (count > 0) {
            *filled += (size_t)count;
            continue;
        }
        if (count < 0 && errno == EINTR) {
            continue;
        }
        if (count < 0 && errno == EAGAIN) {
            if (chunk_size > 1) {
                chunk_size = 1;
                continue;
            }
            break;
        }
        (void)fcntl(fd, F_SETFL, flags);
        return (-1);
    }

    return (fcntl(fd, F_SETFL, flags));
}

static int drain_blocked_write_pipe(int data_fd, int release_fd, int ready_fd, size_t filled)
{
    unsigned char value = 0;
    if (read_retry(release_fd, &value, sizeof(value), NULL) != sizeof(value)) {
        return (-1);
    }
    if (read_retry(data_fd, &value, sizeof(value), NULL) != sizeof(value) ||
        value != PIPE_FILL_BYTE) {
        return (-1);
    }

    const unsigned char ready = 0x5c;
    if (write(ready_fd, &ready, sizeof(ready)) != sizeof(ready)) {
        return (-1);
    }

    size_t filler_remaining = filled - 1;
    int committed_seen = 0;
    unsigned char buffer[PIPE_FILL_CHUNK_SIZE];
    for (;;) {
        ssize_t count = read_retry(data_fd, buffer, sizeof(buffer), NULL);
        if (count < 0) {
            return (-1);
        }
        if (count == 0) {
            break;
        }
        for (ssize_t i = 0; i < count; i++) {
            if (filler_remaining > 0) {
                if (buffer[i] != PIPE_FILL_BYTE) {
                    return (-1);
                }
                filler_remaining--;
            } else if (!committed_seen) {
                if (buffer[i] != COMMITTED_WRITE_BYTE) {
                    return (-1);
                }
                committed_seen = 1;
            } else {
                return (-1);
            }
        }
    }

    return (filler_remaining == 0 && committed_seen ? 0 : -1);
}

static int test_blocked_write(void)
{
    int data_pipe[2];
    int release_pipe[2];
    int ready_pipe[2];
    CHECK(40, pipe(data_pipe) == 0);
    CHECK(41, pipe(release_pipe) == 0);
    CHECK(42, pipe(ready_pipe) == 0);

    size_t filled = 0;
    CHECK(43, fill_pipe(data_pipe[1], &filled) == 0);
    CHECK(44, filled > 0);

    pid_t drainer = fork();
    CHECK(45, drainer >= 0);
    if (drainer == 0) {
        (void)close(data_pipe[1]);
        (void)close(release_pipe[1]);
        (void)close(ready_pipe[0]);
        int status = drain_blocked_write_pipe(data_pipe[0], release_pipe[0], ready_pipe[1], filled);
        (void)close(data_pipe[0]);
        (void)close(release_pipe[0]);
        (void)close(ready_pipe[1]);
        _exit(status == 0 ? 0 : 1);
    }

    pid_t interrupter = fork();
    CHECK(46, interrupter >= 0);
    if (interrupter == 0) {
        (void)close(data_pipe[0]);
        (void)close(data_pipe[1]);
        (void)close(release_pipe[0]);
        (void)close(release_pipe[1]);
        (void)close(ready_pipe[0]);
        (void)close(ready_pipe[1]);
        _exit(delay_ms(10) == 0 ? 0 : 1);
    }

    CHECK(47, close(data_pipe[0]) == 0);
    CHECK(48, close(release_pipe[0]) == 0);
    CHECK(49, close(ready_pipe[1]) == 0);

    sig_atomic_t signals_before = sigchld_count;
    const unsigned char cancelled = CANCELLED_WRITE_BYTE;
    errno = 0;
    ssize_t count = write(data_pipe[1], &cancelled, sizeof(cancelled));
    int write_errno = errno;
    int failure = 0;
    if (count != -1) {
        failure = 50;
    } else if (write_errno != EINTR) {
        failure = 51;
    } else if (sigchld_count <= signals_before) {
        failure = 52;
    }

    const unsigned char release = 0x6d;
    if (write(release_pipe[1], &release, sizeof(release)) != sizeof(release) && failure == 0) {
        failure = 53;
    }

    unsigned char ready = 0;
    count = read_retry(ready_pipe[0], &ready, sizeof(ready), NULL);
    if (count != sizeof(ready) && failure == 0) {
        failure = 54;
    } else if (ready != 0x5c && failure == 0) {
        failure = 55;
    }

    if (failure == 0) {
        const unsigned char committed = COMMITTED_WRITE_BYTE;
        if (write(data_pipe[1], &committed, sizeof(committed)) != sizeof(committed)) {
            failure = 56;
        }
    }

    (void)close(data_pipe[1]);
    (void)close(release_pipe[1]);
    (void)close(ready_pipe[0]);
    int interrupter_succeeded = wait_success(interrupter);
    int drainer_succeeded = wait_success(drainer);
    if (failure != 0) {
        return (failure);
    }
    CHECK(57, interrupter_succeeded);
    CHECK(58, drainer_succeeded);
    return (0);
}

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    struct sigaction action = {0};
    action.sa_handler = handle_sigchld;
    CHECK(1, sigemptyset(&action.sa_mask) == 0);
    CHECK(2, sigaction(SIGCHLD, &action, NULL) == 0);

    int signal_pipe[2];
    int data_pipe[2];
    int control_pipe[2];
    int ready_pipe[2];
    CHECK(30, pipe(signal_pipe) == 0);
    signal_rpc_fd = signal_pipe[1];
    CHECK(3, pipe(data_pipe) == 0);
    CHECK(4, pipe(control_pipe) == 0);
    CHECK(5, pipe(ready_pipe) == 0);

    pid_t writer = fork();
    CHECK(6, writer >= 0);
    if (writer == 0) {
        const unsigned char first_data = 0x5a;
        const unsigned char second_data = 0x6b;
        const unsigned char control = 0xc3;
        unsigned char ready = 0;

        (void)close(data_pipe[0]);
        (void)close(control_pipe[0]);
        (void)close(ready_pipe[1]);
        int status = delay_ms(50);
        if (status == 0 &&
            write(data_pipe[1], &first_data, sizeof(first_data)) != sizeof(first_data)) {
            status = -1;
        }
        if (status == 0 && read(ready_pipe[0], &ready, sizeof(ready)) != sizeof(ready)) {
            status = -1;
        }
        if (status == 0 && delay_ms(50) != 0) {
            status = -1;
        }
        if (status == 0 &&
            write(data_pipe[1], &second_data, sizeof(second_data)) != sizeof(second_data)) {
            status = -1;
        }
        if (status == 0 && write(control_pipe[1], &control, sizeof(control)) != sizeof(control)) {
            status = -1;
        }
        (void)close(data_pipe[1]);
        (void)close(control_pipe[1]);
        (void)close(ready_pipe[0]);
        _exit(status == 0 ? 0 : 1);
    }

    // Forked after the writer so that only the three closes below, and not a fork(), have to fit
    // inside this child's 10 ms fuse for the signal to land while the parent is blocked in read().
    pid_t interrupter = fork();
    CHECK(7, interrupter >= 0);
    if (interrupter == 0) {
        (void)close(data_pipe[0]);
        (void)close(data_pipe[1]);
        (void)close(control_pipe[0]);
        (void)close(control_pipe[1]);
        (void)close(ready_pipe[0]);
        (void)close(ready_pipe[1]);
        _exit(delay_ms(10) == 0 ? 0 : 1);
    }

    CHECK(8, close(data_pipe[1]) == 0);
    CHECK(9, close(control_pipe[1]) == 0);
    CHECK(10, close(ready_pipe[0]) == 0);

    unsigned char first_data = 0;
    int interrupted = 0;
    ssize_t count = read_retry(data_pipe[0], &first_data, sizeof(first_data), &interrupted);
    CHECK(11, interrupted);
    CHECK(12, sigchld_count > 0);
    CHECK(13, count == sizeof(first_data));
    CHECK(14, first_data == 0x5a);
    CHECK(31, signal_rpc_status == 1);

    unsigned char signal_data = 0;
    CHECK(32, read(signal_pipe[0], &signal_data, sizeof(signal_data)) == sizeof(signal_data));
    CHECK(33, signal_data == SIGNAL_RPC_BYTE);

    const unsigned char ready = 0xa5;
    CHECK(15, write(ready_pipe[1], &ready, sizeof(ready)) == sizeof(ready));

    unsigned char control = 0;
    count = read_retry(control_pipe[0], &control, sizeof(control), NULL);
    int failure = 0;
    if (count != sizeof(control)) {
        failure = 16;
    } else if (control != 0xc3) {
        failure = 17;
    }
    if (failure != 0) {
        sigset_t blocked;
        (void)sigemptyset(&blocked);
        (void)sigaddset(&blocked, SIGCHLD);
        (void)sigprocmask(SIG_BLOCK, &blocked, NULL);
        (void)wait_success(interrupter);
        (void)wait_success(writer);
        (void)close(data_pipe[0]);
        (void)close(control_pipe[0]);
        (void)close(ready_pipe[1]);
        return (failure);
    }

    unsigned char second_data = 0;
    count = read_retry(data_pipe[0], &second_data, sizeof(second_data), NULL);
    CHECK(18, count == sizeof(second_data));
    CHECK(19, second_data == 0x6b);

    CHECK(20, wait_success(interrupter));
    CHECK(21, wait_success(writer));
    CHECK(22, close(data_pipe[0]) == 0);
    CHECK(23, close(control_pipe[0]) == 0);
    CHECK(24, close(ready_pipe[1]) == 0);

    int write_test_status = test_blocked_write();
    if (write_test_status != 0) {
        return (write_test_status);
    }

    sigset_t blocked;
    CHECK(25, sigemptyset(&blocked) == 0);
    CHECK(26, sigaddset(&blocked, SIGCHLD) == 0);
    CHECK(27, sigprocmask(SIG_BLOCK, &blocked, NULL) == 0);

    pid_t probe = fork();
    CHECK(28, probe >= 0);
    if (probe == 0) {
        _exit(0);
    }

    CHECK(29, wait_success(probe));
    signal_rpc_fd = -1;
    CHECK(34, close(signal_pipe[0]) == 0);
    CHECK(35, close(signal_pipe[1]) == 0);
    return (0);
}
