/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Regression test: a fork()'d child must be able to (re)initialize an inherited
 * pthread mutex / condition variable.
 *
 * This guards the post-fork lock-rebuild path used by runtimes such as CPython
 * (see "Background"). POSIX leaves re-initialization of an already-initialized
 * mutex/cond UNDEFINED; Nanvix defines it as an idempotent "reset", and this
 * test asserts that Nanvix-specific behavior. It currently passes on the pinned
 * Nanvix sysroot and will fail if the re-init regression is reintroduced.
 *
 * Background
 * ----------
 * On Nanvix, libposix keeps address-keyed registries (MUTEXES / CONDITIONS)
 * that map a pthread_mutex_t* / pthread_cond_t* to kernel synchronization
 * objects. A fork()'d child inherits those registry entries via copy-on-write,
 * but the kernel intentionally drops the per-process sync objects across fork
 * and recreates them lazily. Consequently the child must be able to
 * (re)initialize a mutex/cond at an address that is still present in the
 * inherited registry.
 *
 * If pthread_mutex_init() / pthread_cond_init() reject an already-registered
 * address, the child cannot rebuild its locks. This is exactly what runtimes
 * such as CPython do after fork(): they re-create their locks (e.g. a fresh
 * GIL) in the child. The bug surfaced there as
 * "create_gil: PyMUTEX_INIT failed" and aborted the forked interpreter.
 *
 * What this program demonstrates
 * -------------------------------------------------------
 *   PTHREAD-START     - the test starts.
 *   PARENT-INIT-OK    - the parent initialized a mutex + cond and locked/
 *                       unlocked the mutex.
 *   CHILD-REINIT-OK   - the child re-initialized the SAME (inherited) mutex
 *                       and cond addresses successfully.  <-- on a correct
 *                       system; on the buggy system the re-init returns an
 *                       error and this FAILS.
 *   CHILD-USE-OK      - the child could lock/unlock the re-initialized mutex.
 *   ok                - the whole sequence succeeded.
 *
 * Correct behavior : all markers print and the process exits 0.
 * Buggy behavior   : the child's status byte is non-zero, the parent prints
 *                    CHILD-REINIT-FAILED and exits non-zero.
 *
 * A one-byte status is passed from the child to the parent over a pipe();
 * pipes themselves work on Nanvix, so they are a reliable side channel here.
 */

#include <pthread.h>
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

/* Inherited by the child via copy-on-write at the SAME addresses. */
static pthread_mutex_t mutex;
static pthread_cond_t cond;

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    emit("PTHREAD-START\n");

    /* Parent: initialize a mutex + cond, then exercise the mutex. */
    if (pthread_mutex_init(&mutex, NULL) != 0) {
        emit("PARENT-MUTEX-INIT-FAILED\n");
        return (1);
    }
    if (pthread_cond_init(&cond, NULL) != 0) {
        emit("PARENT-COND-INIT-FAILED\n");
        return (1);
    }
    if (pthread_mutex_lock(&mutex) != 0 || pthread_mutex_unlock(&mutex) != 0) {
        emit("PARENT-MUTEX-USE-FAILED\n");
        return (1);
    }
    emit("PARENT-INIT-OK\n");

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
        /* Child: rebuild its synchronization primitives, mirroring what a
           runtime does after fork(). The addresses are the inherited ones. */
        (void)close(p[0]);
        unsigned char st = 0;

        if (pthread_mutex_init(&mutex, NULL) != 0) {
            st = 1; /* re-init of inherited mutex rejected */
        } else if (pthread_cond_init(&cond, NULL) != 0) {
            st = 2; /* re-init of inherited cond rejected */
        } else if (pthread_mutex_lock(&mutex) != 0 ||
                   pthread_mutex_unlock(&mutex) != 0) {
            st = 3; /* re-initialized mutex not usable */
        }

        (void)write(p[1], &st, 1);
        (void)close(p[1]);
        _exit(st == 0 ? 0 : 1);
    }

    /* Parent: collect the child's result. */
    (void)close(p[1]);
    unsigned char st = 0xff;
    ssize_t n = read(p[0], &st, 1);
    (void)close(p[0]);

    int status = 0;
    if (waitpid(pid, &status, 0) != pid) {
        emit("WAITPID-FAILED\n");
        return (1);
    }
    if (n != 1) {
        emit("CHILD-REPORT-FAILED\n");
        return (1);
    }
    if (st != 0) {
        if (st == 3) {
            emit("CHILD-MUTEX-USE-FAILED\n");
        } else {
            emit("CHILD-REINIT-FAILED\n");
        }
        return (1);
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != 0) {
        emit("CHILD-NONZERO-EXIT\n");
        return (1);
    }
    emit("CHILD-REINIT-OK\n");
    emit("CHILD-USE-OK\n");

    emit("ok\n");
    return (0);
}
