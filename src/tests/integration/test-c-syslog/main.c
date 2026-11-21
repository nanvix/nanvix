/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#include <assert.h>
#include <errno.h>
#include <string.h>
#include <syslog.h>
#include <unistd.h>

int main(void)
{
    int initial_mask;
    char long_prefix[5001];
    int error_pipe[2];
    int saved_stderr;
    char error_output[128];
    ssize_t error_output_len;

    initial_mask = setlogmask(0);
    assert(initial_mask == LOG_UPTO(LOG_DEBUG));
    assert(setlogmask(LOG_MASK(LOG_ERR)) == initial_mask);

    openlog("syslog-test", LOG_PID | LOG_PERROR, LOG_LOCAL0);
    syslog(LOG_INFO, "filtered");
    assert(setlogmask(0) == LOG_MASK(LOG_ERR));

    memset(long_prefix, 'x', sizeof(long_prefix) - 1);
    long_prefix[sizeof(long_prefix) - 1] = '\0';
    errno = ENOENT;
    syslog(LOG_ERR, "%s missing: %m (%d)", long_prefix, 42);
    assert(errno == ENOENT);

    assert(pipe(error_pipe) == 0);
    saved_stderr = dup(STDERR_FILENO);
    assert(saved_stderr >= 0);
    assert(dup2(error_pipe[1], STDERR_FILENO) == STDERR_FILENO);
    assert(close(error_pipe[1]) == 0);
    syslog(LOG_ERR, "byte:%c", 0xff);
    assert(dup2(saved_stderr, STDERR_FILENO) == STDERR_FILENO);
    assert(close(saved_stderr) == 0);
    error_output_len = read(error_pipe[0], error_output, sizeof(error_output));
    assert(error_output_len >= 2);
    assert(error_output[error_output_len - 2] == (char)0xff);
    assert(error_output[error_output_len - 1] == '\n');
    assert(close(error_pipe[0]) == 0);

    closelog();
    assert(setlogmask(0) == LOG_MASK(LOG_ERR));
    assert(setlogmask(initial_mask) == LOG_MASK(LOG_ERR));

    assert(write(STDOUT_FILENO, "ok", 2) == 2);
    return (0);
}
