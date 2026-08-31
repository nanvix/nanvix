/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

//==================================================================================================
// Imports
//==================================================================================================

#include <assert.h>
#include <fcntl.h>
#include <poll.h>
#include <termios.h>
#include <unistd.h>

//==================================================================================================
// Standalone Functions
//==================================================================================================

// Tests the system console independently of the standard descriptor slots.
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

    int console = open("/dev/console", O_WRONLY | O_CREAT | O_TRUNC, 0);
    assert(console >= 0);
    assert(isatty(console) == 1);

    struct termios attrs;
    assert(tcgetattr(console, &attrs) == 0);

    struct pollfd output = {
        .fd = console,
        .events = POLLOUT,
        .revents = 0,
    };
    assert(poll(&output, 1, 0) == 1);
    assert(output.revents & POLLOUT);
    assert(write(console, ".", 1) == 1);

    assert(close(console) == 0);
    assert(dup2(saved[0], STDIN_FILENO) == STDIN_FILENO);
    assert(dup2(saved[1], STDOUT_FILENO) == STDOUT_FILENO);
    assert(dup2(saved[2], STDERR_FILENO) == STDERR_FILENO);
    assert(close(saved[0]) == 0);
    assert(close(saved[1]) == 0);
    assert(close(saved[2]) == 0);
    assert(close(redirected) == 0);
    assert(unlink("terminal-redirection.tmp") == 0);
}
