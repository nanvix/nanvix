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
#include <poll.h>
#include <sys/wait.h>
#include <termios.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests named terminal devices independently of the standard descriptor slots.
void test_terminal_devices(void)
{
    int saved[3] = {dup(STDIN_FILENO), dup(STDOUT_FILENO), dup(STDERR_FILENO)};
    assert(saved[0] >= 0);
    assert(saved[1] >= 0);
    assert(saved[2] >= 0);

    int redirected = open("terminal-redirection.tmp", O_CREAT | O_RDWR, 0600);
    assert(redirected >= 0);
    assert(dup2(redirected, STDIN_FILENO) == STDIN_FILENO);
    assert(dup2(redirected, STDOUT_FILENO) == STDOUT_FILENO);
    assert(dup2(redirected, STDERR_FILENO) == STDERR_FILENO);

    int tty = open("/dev/tty", O_RDWR);
    int console = open("/dev/console", O_WRONLY | O_CREAT | O_TRUNC, 0);
    assert(tty >= 0);
    assert(console >= 0);
    assert(isatty(tty) == 1);
    assert(isatty(console) == 1);

    struct termios attrs;
    assert(tcgetattr(tty, &attrs) == 0);

    struct pollfd output = {
        .fd = console,
        .events = POLLOUT,
        .revents = 0,
    };
    assert(poll(&output, 1, 0) == 1);
    assert(output.revents & POLLOUT);
    assert(write(console, ".", 1) == 1);

    assert(close(tty) == 0);
    assert(close(console) == 0);
    assert(dup2(saved[0], STDIN_FILENO) == STDIN_FILENO);
    assert(dup2(saved[1], STDOUT_FILENO) == STDOUT_FILENO);
    assert(dup2(saved[2], STDERR_FILENO) == STDERR_FILENO);
    assert(close(saved[0]) == 0);
    assert(close(saved[1]) == 0);
    assert(close(saved[2]) == 0);
    assert(close(redirected) == 0);
    assert(unlink("terminal-redirection.tmp") == 0);

    pid_t child = fork();
    assert(child >= 0);
    if (child == 0) {
        assert(setsid() >= 0);
        errno = 0;
        assert(open("/dev/tty", O_RDONLY) == -1);
        assert(errno == ENXIO);
        int detached_console = open("/dev/console", O_RDONLY);
        assert(detached_console >= 0);
        assert(close(detached_console) == 0);
        _exit(0);
    }

    int status = 0;
    assert(waitpid(child, &status, 0) == child);
    assert(WIFEXITED(status));
    assert(WEXITSTATUS(status) == 0);
}
