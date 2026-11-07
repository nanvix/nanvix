/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Regression test for the signal state that execv() carries into the new process image.
 *
 * Background
 * ----------
 * POSIX requires the new process image to inherit the pending signals of the process, and its
 * initial thread to inherit the signal mask and the pending signals of the thread that called
 * execv(). Signals set to be caught are reset to SIG_DFL, because their handlers live in the old
 * image, while signals set to SIG_IGN stay ignored. Setting a signal action to SIG_DFL discards a
 * pending signal whose default action is to ignore it, whether or not it is blocked (XSH 2.4.3).
 *
 * How it works (single ELF, two roles)
 * ------------------------------------
 * This program is bundled into the test ramfs a second time as `/bin/prog` and plays two roles
 * selected by argv:
 *   - Setup role (argc < 2): catches SIGUSR1, SIGUSR2, and SIGURG, ignores SIGHUP, blocks the
 *     caught signals, raises them, checks that they are pending, and then replaces its image with
 *     /bin/prog, passing argv[1] == "exec-signals-child".
 *   - Verify role: checks the signal state inherited by the new image (see below).
 *
 * Checks in the new image (all must pass)
 * ---------------------------------------
 *   MASK-OK     SIGUSR1, SIGUSR2, and SIGURG are still blocked.
 *   RESET-OK    The caught signals are reset to SIG_DFL, while SIGHUP stays ignored.
 *   PENDING-OK  sigpending() reports SIGUSR1 and SIGUSR2, but not SIGURG, which is discarded
 *               because its default action is to ignore it.
 *   DELIVER-OK  Once unblocked under handlers installed by the new image, SIGUSR1 and SIGUSR2 are
 *               delivered exactly once each, and no signal remains pending.
 *
 * Pass/fail (standalone): guest stdout is discarded, so the propagated exit code is authoritative.
 * The image that runs last returns 0 on success and a distinct non-zero code that pinpoints the
 * failing check otherwise: 1-15 for the setup role and 101-124 for the verify role. The emit()
 * markers are diagnostic only.
 */

#include <signal.h>
#include <string.h>
#include <unistd.h>

//==================================================================================================
// Constants
//==================================================================================================

/* Path at which the suite ELF is staged in the test ramfs. */
#define TARGET_PATH "/bin/prog"
/* argv[1] sentinel that selects the verify role of this dual-role ELF. */
#define CHILD_MARKER "exec-signals-child"

/* Returns a distinct non-zero code identifying the failing check. */
#define CHECK(step, cond)  \
    do {                   \
        if (!(cond)) {     \
            return (step); \
        }                  \
    } while (0)

//==================================================================================================
// Global Variables
//==================================================================================================

/* Number of times SIGUSR1 was delivered to this image. */
static volatile sig_atomic_t usr1_count = 0;
/* Number of times SIGUSR2 was delivered to this image. */
static volatile sig_atomic_t usr2_count = 0;

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

/* Counts the deliveries of SIGUSR1 and SIGUSR2. */
static void count_signal(int signum)
{
    if (signum == SIGUSR1) {
        usr1_count++;
    } else if (signum == SIGUSR2) {
        usr2_count++;
    }
}

/* Installs `handler` as the disposition of `signum`. Returns 0 on success and -1 on failure. */
static int set_disposition(int signum, sighandler_t handler)
{
    struct sigaction act;

    memset(&act, 0, sizeof(act));
    act.sa_handler = handler;
    if (sigemptyset(&act.sa_mask) != 0) {
        return (-1);
    }
    return (sigaction(signum, &act, NULL));
}

/* Reports whether the disposition of `signum` is the default action (SIG_DFL). */
static int is_default(int signum)
{
    struct sigaction old;

    if (sigaction(signum, NULL, &old) != 0) {
        return (0);
    }
    return (old.sa_handler == SIG_DFL);
}

/*
 * Setup role: leaves SIGUSR1, SIGUSR2, and SIGURG caught, blocked, and pending, ignores SIGHUP, and
 * replaces this image with the verify role. Returns only on failure.
 */
static int setup(void)
{
    sigset_t set;
    sigset_t pending;

    emit("EXEC-SIGNALS-START\n");

    /* Catch the signals, so that raising them while they are blocked leaves them pending. */
    CHECK(1, set_disposition(SIGUSR1, count_signal) == 0);
    CHECK(2, set_disposition(SIGUSR2, count_signal) == 0);
    CHECK(3, set_disposition(SIGURG, count_signal) == 0);
    CHECK(4, set_disposition(SIGHUP, SIG_IGN) == 0);

    /* Block the caught signals and raise them. */
    CHECK(5, sigemptyset(&set) == 0);
    CHECK(6, sigaddset(&set, SIGUSR1) == 0);
    CHECK(7, sigaddset(&set, SIGUSR2) == 0);
    CHECK(8, sigaddset(&set, SIGURG) == 0);
    CHECK(9, sigprocmask(SIG_BLOCK, &set, NULL) == 0);
    CHECK(10, raise(SIGUSR1) == 0);
    CHECK(11, raise(SIGUSR2) == 0);
    CHECK(12, raise(SIGURG) == 0);

    /* The raised signals are pending, and none of them was delivered. */
    CHECK(13, sigpending(&pending) == 0);
    CHECK(14,
          sigismember(&pending, SIGUSR1) == 1 && sigismember(&pending, SIGUSR2) == 1 &&
              sigismember(&pending, SIGURG) == 1 && usr1_count == 0 && usr2_count == 0);
    emit("SETUP-OK\n");

    /* Replace this image with a copy of itself in the verify role. */
    char *const argv[] = {(char *)"prog", (char *)CHILD_MARKER, NULL};
    (void)execv(TARGET_PATH, argv);

    /* Only reached if execv() failed. */
    emit("EXECV-FAILED\n");
    return (15);
}

/* Verify role: checks the signal state inherited by the new image. */
static int verify(void)
{
    sigset_t set;
    sigset_t mask;
    sigset_t pending;
    struct sigaction old;

    emit("EXEC-SIGNALS-CHILD\n");

    /* The signal mask of the calling thread survives the image replacement. */
    CHECK(101, sigprocmask(SIG_BLOCK, NULL, &mask) == 0);
    CHECK(102, sigismember(&mask, SIGUSR1) == 1);
    CHECK(103, sigismember(&mask, SIGUSR2) == 1);
    CHECK(104, sigismember(&mask, SIGURG) == 1);
    emit("MASK-OK\n");

    /* Caught signals are reset to the default action, while ignored signals stay ignored. */
    CHECK(105, is_default(SIGUSR1));
    CHECK(106, is_default(SIGUSR2));
    CHECK(107, is_default(SIGURG));
    CHECK(108, sigaction(SIGHUP, NULL, &old) == 0);
    CHECK(109, old.sa_handler == SIG_IGN);
    emit("RESET-OK\n");

    /*
     * Pending signals survive the image replacement, except SIGURG: its default action is to ignore
     * it, so resetting it to SIG_DFL discards it.
     */
    CHECK(110, sigpending(&pending) == 0);
    CHECK(111, sigismember(&pending, SIGUSR1) == 1);
    CHECK(112, sigismember(&pending, SIGUSR2) == 1);
    CHECK(113, sigismember(&pending, SIGURG) == 0);
    emit("PENDING-OK\n");

    /*
     * The surviving signals are delivered to the handlers of this image once unblocked. Delivery
     * happens before sigprocmask() returns, so both handlers have run by then.
     */
    CHECK(114, set_disposition(SIGUSR1, count_signal) == 0);
    CHECK(115, set_disposition(SIGUSR2, count_signal) == 0);
    CHECK(116, sigemptyset(&set) == 0);
    CHECK(117, sigaddset(&set, SIGUSR1) == 0);
    CHECK(118, sigaddset(&set, SIGUSR2) == 0);
    CHECK(119, sigprocmask(SIG_UNBLOCK, &set, NULL) == 0);
    CHECK(120, usr1_count == 1);
    CHECK(121, usr2_count == 1);

    /*
     * sigpending() reports only blocked signals, so block the delivered signals again to check that
     * their delivery consumed them.
     */
    CHECK(122, sigprocmask(SIG_BLOCK, &set, NULL) == 0);
    CHECK(123, sigpending(&pending) == 0);
    CHECK(124, sigisemptyset(&pending) == 1);
    emit("DELIVER-OK\n");

    emit("ok\n");
    return (0);
}

int main(int argc, char *argv[])
{
    /* Verify role: this image was installed by the setup role through execv(). */
    if (argc >= 2 && strcmp(argv[1], CHILD_MARKER) == 0) {
        return (verify());
    }

    return (setup());
}
