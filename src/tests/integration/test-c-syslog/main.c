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

    closelog();
    assert(setlogmask(0) == LOG_MASK(LOG_ERR));
    assert(setlogmask(initial_mask) == LOG_MASK(LOG_ERR));

    assert(write(STDOUT_FILENO, "ok", 2) == 2);
    return (0);
}
