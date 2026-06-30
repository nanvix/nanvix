/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <termios.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests whether tcgetattr() performs a real query against the terminal, overwriting the caller's
// struct with the actual attributes instead of leaving it untouched (the silent-corruption hazard
// of the former no-op stub).
static void test_tcgetattr_populates(void)
{
    fprintf(stderr, "testing tcgetattr() populates attributes ... ");

    // Pre-fill the struct with a sentinel pattern so a no-op implementation that leaves it
    // untouched would be detected by the assertions below.
    struct termios tio;
    memset(&tio, 0xA5, sizeof(tio));

    assert(tcgetattr(STDIN_FILENO, &tio) == 0);

    // The interactive console comes up in canonical mode with echo enabled.
    assert((tio.c_lflag & ICANON) != 0);
    assert((tio.c_lflag & ECHO) != 0);

    // Characters are 8 bits wide and the receiver is enabled.
    assert((tio.c_cflag & CSIZE) == CS8);
    assert((tio.c_cflag & CREAD) != 0);

    // The default console line speed is 38400 baud on both directions.
    assert(tio.c_ispeed == (speed_t)B38400);
    assert(tio.c_ospeed == (speed_t)B38400);

    fprintf(stderr, "passed\n");
}

// Tests whether tcsetattr() actually applies the requested attributes and that a subsequent
// tcgetattr() reflects them (round-trip consistency), proving the call is no longer a no-op.
static void test_tcsetattr_roundtrip(void)
{
    fprintf(stderr, "testing tcsetattr() round-trip ... ");

    struct termios original;
    assert(tcgetattr(STDIN_FILENO, &original) == 0);

    // Switch to a raw-ish mode: disable canonical input and echo, and adjust the non-canonical read
    // parameters.
    struct termios raw = original;
    raw.c_lflag &= ~(tcflag_t)(ICANON | ECHO);
    raw.c_cc[VMIN] = 4;
    raw.c_cc[VTIME] = 7;
    assert(tcsetattr(STDIN_FILENO, TCSANOW, &raw) == 0);

    // The applied change must be observable through a fresh query.
    struct termios readback;
    memset(&readback, 0xA5, sizeof(readback));
    assert(tcgetattr(STDIN_FILENO, &readback) == 0);
    assert((readback.c_lflag & ICANON) == 0);
    assert((readback.c_lflag & ECHO) == 0);
    assert(readback.c_cc[VMIN] == 4);
    assert(readback.c_cc[VTIME] == 7);

    // Restore the captured attributes and confirm the restore round-trips: a fresh query must
    // return exactly the bytes we captured, without assuming any particular default console mode.
    assert(tcsetattr(STDIN_FILENO, TCSANOW, &original) == 0);
    assert(tcgetattr(STDIN_FILENO, &readback) == 0);
    assert(memcmp(&readback, &original, sizeof(original)) == 0);

    fprintf(stderr, "passed\n");
}

// Tests whether the terminal attributes are shared across the standard streams: a change applied
// through one console descriptor is observable through the others.
static void test_tcsetattr_shared_across_streams(void)
{
    fprintf(stderr, "testing shared terminal state across stdin/stdout/stderr ... ");

    struct termios original;
    assert(tcgetattr(STDOUT_FILENO, &original) == 0);

    struct termios modified = original;
    modified.c_lflag &= ~(tcflag_t)ECHO;
    assert(tcsetattr(STDOUT_FILENO, TCSANOW, &modified) == 0);

    // A change applied via stdout is visible through stdin and stderr.
    struct termios via_stdin;
    struct termios via_stderr;
    assert(tcgetattr(STDIN_FILENO, &via_stdin) == 0);
    assert(tcgetattr(STDERR_FILENO, &via_stderr) == 0);
    assert((via_stdin.c_lflag & ECHO) == 0);
    assert((via_stderr.c_lflag & ECHO) == 0);

    // Restore the original attributes.
    assert(tcsetattr(STDOUT_FILENO, TCSANOW, &original) == 0);

    fprintf(stderr, "passed\n");
}

// Tests the optional_actions argument of tcsetattr(): the three legal values are accepted and any
// other value is rejected with EINVAL, even on a valid terminal descriptor.
static void test_tcsetattr_optional_actions(void)
{
    fprintf(stderr, "testing tcsetattr() optional_actions validation ... ");

    struct termios tio;
    assert(tcgetattr(STDIN_FILENO, &tio) == 0);

    // All three POSIX actions are accepted on a terminal descriptor.
    assert(tcsetattr(STDIN_FILENO, TCSANOW, &tio) == 0);
    assert(tcsetattr(STDIN_FILENO, TCSADRAIN, &tio) == 0);
    assert(tcsetattr(STDIN_FILENO, TCSAFLUSH, &tio) == 0);

    // An unrecognized action is rejected with EINVAL.
    errno = 0;
    assert(tcsetattr(STDIN_FILENO, 0x7fff, &tio) == -1);
    assert(errno == EINVAL);

    fprintf(stderr, "passed\n");
}

// Tests that tcgetattr()/tcsetattr() on an invalid descriptor fail with EBADF.
static void test_tc_attr_ebadf(void)
{
    fprintf(stderr, "testing tcgetattr()/tcsetattr() EBADF ... ");

    // -1 is never a valid descriptor, so it deterministically yields EBADF. A fixed high number
    // (e.g. 4096) could instead collide with a legitimately open descriptor.
    const int bad_fd = -1;
    struct termios tio;
    memset(&tio, 0, sizeof(tio));

    errno = 0;
    assert(tcgetattr(bad_fd, &tio) == -1);
    assert(errno == EBADF);

    errno = 0;
    assert(tcsetattr(bad_fd, TCSANOW, &tio) == -1);
    assert(errno == EBADF);

    fprintf(stderr, "passed\n");
}

// Tests that tcgetattr()/tcsetattr() on a valid descriptor that is not a terminal (a pipe end) fail
// with ENOTTY.
static void test_tc_attr_enotty(void)
{
    fprintf(stderr, "testing tcgetattr()/tcsetattr() ENOTTY ... ");

    int fds[2];
    assert(pipe(fds) == 0);

    struct termios tio;
    memset(&tio, 0, sizeof(tio));

    errno = 0;
    assert(tcgetattr(fds[0], &tio) == -1);
    assert(errno == ENOTTY);

    errno = 0;
    assert(tcsetattr(fds[1], TCSANOW, &tio) == -1);
    assert(errno == ENOTTY);

    assert(close(fds[0]) == 0);
    assert(close(fds[1]) == 0);

    fprintf(stderr, "passed\n");
}

/**
 * @brief Tests the terminal-attribute interfaces tcgetattr() and tcsetattr().
 *
 * @param argc Number of command-line arguments (unused).
 * @param argv List of command-line arguments (unused).
 *
 * @returns Always returns zero. If a test fails, the program aborts.
 */
int main(int argc, const char *argv[])
{
    (void)argc;
    (void)argv;

    test_tcgetattr_populates();
    test_tcsetattr_roundtrip();
    test_tcsetattr_shared_across_streams();
    test_tcsetattr_optional_actions();
    test_tc_attr_ebadf();
    test_tc_attr_enotty();

    return 0;
}
