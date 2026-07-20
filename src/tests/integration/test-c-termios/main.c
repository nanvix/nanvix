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

// Tests tcdrain(): succeeds on a terminal (the console has no output queue to drain) and fails with
// EBADF / ENOTTY on invalid / non-terminal descriptors.
static void test_tcdrain(void)
{
    fprintf(stderr, "testing tcdrain() ... ");

    // A terminal descriptor drains successfully.
    assert(tcdrain(STDIN_FILENO) == 0);

    // -1 is never a valid descriptor, so it deterministically yields EBADF.
    errno = 0;
    assert(tcdrain(-1) == -1);
    assert(errno == EBADF);

    // A pipe end is a valid descriptor that is not a terminal.
    int fds[2];
    assert(pipe(fds) == 0);
    errno = 0;
    assert(tcdrain(fds[0]) == -1);
    assert(errno == ENOTTY);
    assert(close(fds[0]) == 0);
    assert(close(fds[1]) == 0);

    fprintf(stderr, "passed\n");
}

// Tests tcsendbreak(): succeeds on a terminal (the console has no serial line to signal) regardless
// of the duration argument, and fails with EBADF / ENOTTY on invalid / non-terminal descriptors.
static void test_tcsendbreak(void)
{
    fprintf(stderr, "testing tcsendbreak() ... ");

    // A terminal descriptor accepts a break request; the duration is ignored.
    assert(tcsendbreak(STDIN_FILENO, 0) == 0);
    assert(tcsendbreak(STDIN_FILENO, 100) == 0);

    errno = 0;
    assert(tcsendbreak(-1, 0) == -1);
    assert(errno == EBADF);

    int fds[2];
    assert(pipe(fds) == 0);
    errno = 0;
    assert(tcsendbreak(fds[0], 0) == -1);
    assert(errno == ENOTTY);
    assert(close(fds[0]) == 0);
    assert(close(fds[1]) == 0);

    fprintf(stderr, "passed\n");
}

// Tests tcflush(): accepts the three POSIX queue selectors on a terminal, rejects any other
// selector with EINVAL, and fails with EBADF / ENOTTY on invalid / non-terminal descriptors.
static void test_tcflush(void)
{
    fprintf(stderr, "testing tcflush() ... ");

    // All three POSIX queue selectors are accepted on a terminal descriptor.
    assert(tcflush(STDIN_FILENO, TCIFLUSH) == 0);
    assert(tcflush(STDIN_FILENO, TCOFLUSH) == 0);
    assert(tcflush(STDIN_FILENO, TCIOFLUSH) == 0);

    // An unrecognized selector is rejected with EINVAL, even on a valid terminal.
    errno = 0;
    assert(tcflush(STDIN_FILENO, 0x7fff) == -1);
    assert(errno == EINVAL);

    // A valid selector on an invalid descriptor yields EBADF.
    errno = 0;
    assert(tcflush(-1, TCIFLUSH) == -1);
    assert(errno == EBADF);

    // The descriptor is validated before the selector: an invalid descriptor paired with an
    // invalid selector still reports EBADF, not EINVAL.
    errno = 0;
    assert(tcflush(-1, 0x7fff) == -1);
    assert(errno == EBADF);

    // A valid selector on a non-terminal descriptor yields ENOTTY.
    int fds[2];
    assert(pipe(fds) == 0);
    errno = 0;
    assert(tcflush(fds[0], TCIFLUSH) == -1);
    assert(errno == ENOTTY);
    assert(close(fds[0]) == 0);
    assert(close(fds[1]) == 0);

    fprintf(stderr, "passed\n");
}

// Tests tcflow(): accepts the four POSIX actions on a terminal, rejects any other action with
// EINVAL, and fails with EBADF / ENOTTY on invalid / non-terminal descriptors.
static void test_tcflow(void)
{
    fprintf(stderr, "testing tcflow() ... ");

    // All four POSIX actions are accepted on a terminal descriptor.
    assert(tcflow(STDIN_FILENO, TCOOFF) == 0);
    assert(tcflow(STDIN_FILENO, TCOON) == 0);
    assert(tcflow(STDIN_FILENO, TCIOFF) == 0);
    assert(tcflow(STDIN_FILENO, TCION) == 0);

    // An unrecognized action is rejected with EINVAL, even on a valid terminal.
    errno = 0;
    assert(tcflow(STDIN_FILENO, 0x7fff) == -1);
    assert(errno == EINVAL);

    // A valid action on an invalid descriptor yields EBADF.
    errno = 0;
    assert(tcflow(-1, TCOON) == -1);
    assert(errno == EBADF);

    // The descriptor is validated before the action: an invalid descriptor paired with an
    // invalid action still reports EBADF, not EINVAL.
    errno = 0;
    assert(tcflow(-1, 0x7fff) == -1);
    assert(errno == EBADF);

    // A valid action on a non-terminal descriptor yields ENOTTY.
    int fds[2];
    assert(pipe(fds) == 0);
    errno = 0;
    assert(tcflow(fds[0], TCOON) == -1);
    assert(errno == ENOTTY);
    assert(close(fds[0]) == 0);
    assert(close(fds[1]) == 0);

    fprintf(stderr, "passed\n");
}

// Tests the cfgetispeed()/cfgetospeed()/cfsetispeed()/cfsetospeed() line-speed accessors. These
// operate purely on the caller's struct termios: the getters read the stored fields, the setters
// store a requested baud independently per direction, and an unsupported baud is rejected with
// EINVAL without disturbing the struct.
static void test_cfspeed_getset(void)
{
    fprintf(stderr, "testing cfget/cfset i/o speed ... ");

    struct termios tio;
    memset(&tio, 0, sizeof(tio));

    // The getters return the actual stored fields, not a hardcoded constant. Distinct values for
    // the two directions catch a getter that reads the wrong field.
    tio.c_ispeed = B38400;
    tio.c_ospeed = B19200;
    assert(cfgetispeed(&tio) == (speed_t)B38400);
    assert(cfgetospeed(&tio) == (speed_t)B19200);

    // The setters store the requested baud and round-trip through the getters.
    assert(cfsetispeed(&tio, B9600) == 0);
    assert(cfsetospeed(&tio, B115200) == 0);
    assert(cfgetispeed(&tio) == (speed_t)B9600);
    assert(cfgetospeed(&tio) == (speed_t)B115200);

    // B0 (hang up) is a valid baud rate.
    assert(cfsetispeed(&tio, B0) == 0);
    assert(cfgetispeed(&tio) == (speed_t)B0);

    // Setting one direction must not disturb the other.
    assert(cfsetispeed(&tio, B2400) == 0);
    assert(cfgetospeed(&tio) == (speed_t)B115200);
    assert(cfsetospeed(&tio, B4800) == 0);
    assert(cfgetispeed(&tio) == (speed_t)B2400);

    // An unsupported baud value is rejected with EINVAL and leaves the struct untouched.
    errno = 0;
    assert(cfsetispeed(&tio, 0x1234) == -1);
    assert(errno == EINVAL);
    assert(cfgetispeed(&tio) == (speed_t)B2400);

    // The bare CBAUDEX bit is not itself a baud rate.
    errno = 0;
    assert(cfsetospeed(&tio, CBAUDEX) == -1);
    assert(errno == EINVAL);
    assert(cfgetospeed(&tio) == (speed_t)B4800);

    fprintf(stderr, "passed\n");
}

// Tests cfsetspeed(): stores the requested baud in both directions at once, rejects a non-baud
// value with EINVAL, and leaves the struct untouched on rejection.
static void test_cfsetspeed(void)
{
    fprintf(stderr, "testing cfsetspeed() ... ");

    struct termios tio;
    memset(&tio, 0, sizeof(tio));

    // A valid baud is written to both the input and output directions in a single call.
    assert(cfsetspeed(&tio, B9600) == 0);
    assert(cfgetispeed(&tio) == (speed_t)B9600);
    assert(cfgetospeed(&tio) == (speed_t)B9600);

    // A later call overwrites both directions, including B0 (hang up).
    assert(cfsetspeed(&tio, B0) == 0);
    assert(cfgetispeed(&tio) == (speed_t)B0);
    assert(cfgetospeed(&tio) == (speed_t)B0);

    // A non-baud value is rejected with EINVAL and leaves both directions untouched.
    assert(cfsetspeed(&tio, B38400) == 0);
    errno = 0;
    assert(cfsetspeed(&tio, 0x1234) == -1);
    assert(errno == EINVAL);
    assert(cfgetispeed(&tio) == (speed_t)B38400);
    assert(cfgetospeed(&tio) == (speed_t)B38400);

    fprintf(stderr, "passed\n");
}

/**
 * @brief Tests the terminal-control interfaces: tcgetattr()/tcsetattr(), the line-control calls
 * tcdrain()/tcsendbreak()/tcflush()/tcflow(), and the cfget/cfset line-speed accessors.
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
    test_tcdrain();
    test_tcsendbreak();
    test_tcflush();
    test_tcflow();
    test_cfspeed_getset();
    test_cfsetspeed();

    return 0;
}
