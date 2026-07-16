/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Regression test: a fork()'d child must observe its OWN (new) process id.
 *
 * This guards against a stale cached pid in the child (see "Background"). It
 * asserts the POSIX invariant and currently passes on the pinned Nanvix
 * sysroot; it will fail if the cached-pid regression is reintroduced.
 *
 * Background
 * ----------
 * On Nanvix, getpid() is memoized in a per-process cached variable
 * (CACHED_PID) in libposix. A fork()'d child inherits that cache through
 * copy-on-write, so unless the cache is invalidated in the child, getpid()
 * keeps returning the PARENT's pid. POSIX requires that, in the child,
 * getpid() return the child's own (new) pid -- i.e. the same value the
 * parent received from fork() -- and that getppid() return the parent's pid.
 *
 * The stale cache is not merely cosmetic: capability-sensitive kernel calls
 * (e.g. mmap/mprotect during a subsequent execv()) are keyed by the caller's
 * pid, so a child acting under the parent's identity fails those calls with
 * EACCES. This is what broke fork()+exec() of a fresh interpreter.
 *
 * What this program demonstrates
 * -------------------------------------------------------
 *   PID-START          - the test starts.
 *   PARENT-PID-OK      - the parent obtained a non-zero pid.
 *   CHILD-PID-RECEIVED - the child reported its getpid()/getppid() back.
 *   PID-MATCHES-FORK   - the child's getpid() equals fork()'s return value
 *                        AND differs from the parent's pid.  <-- on a correct
 *                        system; on the buggy system the child reports the
 *                        parent's pid, so this check FAILS.
 *   PPID-OK            - the child's getppid() equals the parent's pid.
 *   ok                - the whole sequence succeeded.
 *
 * Correct behavior : all markers print and the process exits 0.
 * Buggy behavior   : prints PID-FORK-MISMATCH and exits non-zero (the child
 *                    reported the parent's pid).
 *
 * The pid values are passed from the child to the parent over a pipe(); pipes
 * themselves work on Nanvix, so they are a reliable side channel here.
 */

#include <sys/wait.h>
#include <unistd.h>

static void emit(const char *s)
{
    const char *p = s;
    while (*p) {
        p++;
    }
    (void)write(STDOUT_FILENO, s, (size_t)(p - s));
}

/* Read exactly `len` bytes into `buf`, looping over short reads. A pipe read()
   may return fewer bytes than requested even for a single small write, so the
   caller must drain it. Returns `len` on success, or a smaller count on EOF, or
   -1 on error. */
static ssize_t read_full(int fd, void *buf, size_t len)
{
    unsigned char *p = (unsigned char *)buf;
    size_t got = 0;
    while (got < len) {
        ssize_t r = read(fd, p + got, len - got);
        if (r < 0) {
            return (-1);
        }
        if (r == 0) {
            break; /* EOF: writer closed before sending the full report. */
        }
        got += (size_t)r;
    }
    return ((ssize_t)got);
}

/* The child's report sent back to the parent over the pipe. */
struct child_report {
    pid_t self; /* child's getpid()  */
    pid_t ppid; /* child's getppid() */
};

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    emit("PID-START\n");

    pid_t parent_pid = getpid();
    if (parent_pid <= 0) {
        emit("PARENT-PID-INVALID\n");
        return (1);
    }
    emit("PARENT-PID-OK\n");

    int p[2];
    if (pipe(p) != 0) {
        emit("PIPE-FAILED\n");
        return (1);
    }

    pid_t pid = fork();
    if (pid < 0) {
        emit("FORK-FAILED\n");
        return (1);
    }

    if (pid == 0) {
        /* Child: report the pids the kernel attributes to us. */
        (void)close(p[0]);
        struct child_report rep;
        rep.self = getpid();
        rep.ppid = getppid();
        ssize_t w = write(p[1], &rep, sizeof(rep));
        (void)close(p[1]);
        _exit(w == (ssize_t)sizeof(rep) ? 0 : 1);
    }

    /* Parent. */
    (void)close(p[1]);
    struct child_report rep = {0, 0};
    ssize_t n = read_full(p[0], &rep, sizeof(rep));
    (void)close(p[0]);

    int status = 0;
    if (waitpid(pid, &status, 0) != pid) {
        emit("WAITPID-FAILED\n");
        return (1);
    }
    if (n != (ssize_t)sizeof(rep)) {
        emit("CHILD-REPORT-FAILED\n");
        return (1);
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        emit("CHILD-NONZERO-EXIT\n");
        return (1);
    }
    emit("CHILD-PID-RECEIVED\n");

    /* The child's getpid() must equal what fork() returned to the parent, and
       must differ from the parent's own pid. On the stale-cache bug the child
       reports the parent's pid instead. */
    if (rep.self != pid || rep.self == parent_pid) {
        emit("PID-FORK-MISMATCH\n");
        return (1);
    }
    emit("PID-MATCHES-FORK\n");

    /* The child's getppid() must be the parent's pid. */
    if (rep.ppid != parent_pid) {
        emit("PPID-MISMATCH\n");
        return (1);
    }
    emit("PPID-OK\n");

    emit("ok\n");
    return (0);
}
