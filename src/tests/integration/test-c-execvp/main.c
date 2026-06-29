/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Acceptance test for execvp(): the PATH search must locate a bare program name
 * (one with no slash) in the directories listed in the PATH environment
 * variable, while a name that contains a slash must be used as a path directly.
 *
 * Background
 * ----------
 * execvp() differs from execv() in how it resolves the program: if `file`
 * contains a slash it is used verbatim, otherwise the directories in PATH are
 * searched in order and the first executable found is run (POSIX). A stub that
 * merely forwards to execv() (treating `file` as a literal path) cannot run a
 * bare program name, which is what this test guards against.
 *
 * How it works (single ELF, two roles)
 * ------------------------------------
 * This program is bundled into the test ramfs a second time as `/bin/prog` and
 * plays two roles selected by argv:
 *   - Target role: when re-exec'd with argv[1] == "execvp-child", it returns the
 *     sentinel exit code 42 and nothing else. Only the program at `/bin/prog`
 *     does this, so observing 42 proves execvp() resolved to it.
 *   - Driver role (argc < 2): it sets PATH and exercises execvp() through
 *     fork()+waitpid(), asserting each child exits with 42.
 *
 * Sub-tests (all must pass)
 * -------------------------
 *   PATH-SEARCH-OK   execvp("prog") with PATH=/bin runs /bin/prog       (core).
 *   PATH-MULTI-OK    execvp("prog") with PATH=/nonexistent:/bin skips
 *                    the missing dir and runs /bin/prog.
 *   SLASH-DIRECT-OK  execvp("/bin/prog") runs it directly even though
 *                    PATH points elsewhere (slash bypasses PATH).
 *   DEFAULT-PATH-OK  execvp("prog") with PATH unset falls back to the
 *                    implementation default path and still runs /bin/prog.
 *   NOTFOUND-OK      execvp() of a name absent from PATH returns -1 with
 *                    errno == ENOENT.
 *
 * Pass/fail (standalone): guest stdout is discarded, so the propagated exit code
 * is authoritative -- main() returns 0 on success and non-zero on the first
 * failure. The emit() markers are diagnostic only.
 */

#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

/* Directory the target is staged in (added to PATH by the driver). */
#define TARGET_DIR "/bin"
/* Bare program name resolved through PATH. */
#define TARGET_NAME "prog"
/* Full path to the target, used by the slash-bypasses-PATH sub-test. */
#define TARGET_PATH (TARGET_DIR "/" TARGET_NAME)
/* argv[1] sentinel that selects the target role of this dual-role ELF. */
#define CHILD_MARKER "execvp-child"
/* Exit code the target returns; the driver asserts the child reports exactly this. */
#define CHILD_EXIT_CODE 42
/* Exit code a child reports if execvp() returned at all (i.e. it failed). */
#define EXEC_FAILED_CODE 127

//==================================================================================================
// Standalone Functions
//==================================================================================================

/* Writes a NUL-terminated diagnostic string to standard output. */
static void emit(const char *s)
{
    const char *p = s;
    while (*p) {
        p++;
    }
    (void)write(STDOUT_FILENO, s, (size_t)(p - s));
}

/*
 * Forks a child that execvp()s `file` (handing it the CHILD_MARKER so the
 * re-exec'd image takes the target role) and waits for it. Returns 0 only if the
 * child exited with CHILD_EXIT_CODE -- i.e. execvp() located and ran /bin/prog.
 * Returns -1 on any failure (fork, waitpid, execvp returning, or a wrong code).
 */
static int expect_exec_ok(const char *file)
{
    pid_t pid = fork();
    if (pid < 0) {
        return (-1);
    }

    if (pid == 0) {
        /* Child: replace the image with the program execvp() resolves. argv[0]
           is conventional; argv[1] selects the target role. */
        char *const cargv[] = {(char *)TARGET_NAME, (char *)CHILD_MARKER, NULL};
        (void)execvp(file, cargv);
        /* Only reached if execvp() failed; report a distinct, non-sentinel code. */
        _exit(EXEC_FAILED_CODE);
    }

    /* Parent: reap the child and require the target's sentinel exit code. */
    int status = 0;
    if (waitpid(pid, &status, 0) != pid) {
        return (-1);
    }
    if (!WIFEXITED(status) || WEXITSTATUS(status) != CHILD_EXIT_CODE) {
        return (-1);
    }
    return (0);
}

int main(int argc, char *argv[])
{
    /* Target role: this image was re-exec'd by the driver via execvp(). Report
       the sentinel and do nothing else. Checked first so the re-exec'd image
       short-circuits before any driver work. */
    if (argc >= 2 && strcmp(argv[1], CHILD_MARKER) == 0) {
        return (CHILD_EXIT_CODE);
    }

    emit("EXECVP-START\n");

    /* 1. A bare name is resolved through PATH. */
    if (setenv("PATH", TARGET_DIR, 1) != 0) {
        emit("SETENV-FAILED\n");
        return (1);
    }
    if (expect_exec_ok(TARGET_NAME) != 0) {
        emit("PATH-SEARCH-FAILED\n");
        return (1);
    }
    emit("PATH-SEARCH-OK\n");

    /* 2. The search walks PATH in order, skipping a missing leading directory. */
    if (setenv("PATH", "/nonexistent:" TARGET_DIR, 1) != 0) {
        emit("SETENV-FAILED\n");
        return (1);
    }
    if (expect_exec_ok(TARGET_NAME) != 0) {
        emit("PATH-MULTI-FAILED\n");
        return (1);
    }
    emit("PATH-MULTI-OK\n");

    /* 3. A name with a slash is used directly and ignores PATH (set to a bogus
          directory here to prove no PATH search happens). */
    if (setenv("PATH", "/nonexistent", 1) != 0) {
        emit("SETENV-FAILED\n");
        return (1);
    }
    if (expect_exec_ok(TARGET_PATH) != 0) {
        emit("SLASH-DIRECT-FAILED\n");
        return (1);
    }
    emit("SLASH-DIRECT-OK\n");

    /* 4. With PATH unset, the search falls back to the implementation default
          path (which includes TARGET_DIR), so a bare name still resolves. */
    if (unsetenv("PATH") != 0) {
        emit("UNSETENV-FAILED\n");
        return (1);
    }
    if (expect_exec_ok(TARGET_NAME) != 0) {
        emit("DEFAULT-PATH-FAILED\n");
        return (1);
    }
    emit("DEFAULT-PATH-OK\n");

    /* 5. A bare name absent from every PATH directory fails with ENOENT. This
          call runs in the driver because execvp() returns on failure. */
    if (setenv("PATH", TARGET_DIR, 1) != 0) {
        emit("SETENV-FAILED\n");
        return (1);
    }
    errno = 0;
    char *const nargv[] = {(char *)"definitely-not-here", NULL};
    if (execvp("definitely-not-here", nargv) != -1) {
        emit("NOTFOUND-RETURN\n");
        return (1);
    }
    if (errno != ENOENT) {
        emit("NOTFOUND-ERRNO\n");
        return (1);
    }
    emit("NOTFOUND-OK\n");

    emit("ok\n");
    return (0);
}
