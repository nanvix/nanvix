/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

/*
 * Integration test for the POSIX signal-mask API:
 *
 *   - sigemptyset / sigfillset / sigisemptyset /
 *     sigaddset / sigdelset / sigismember
 *                     (membership, the empty/full predicates, the largest valid
 *                      signal number, out-of-range EINVAL, and null-set EFAULT)
 *   - sigprocmask     (SIG_BLOCK / SIG_UNBLOCK / SIG_SETMASK, oldset round-trip,
 *                      SIGKILL/SIGSTOP cannot be blocked, EINVAL on bad `how`,
 *                      null-set/null-oldset no-op)
 *   - pthread_sigmask (same semantics as sigprocmask but returns the error
 *                      number directly, and shares the per-thread mask with
 *                      sigprocmask)
 *   - sigsuspend      (argument validation only: a null mask is rejected with
 *                      EINVAL without suspending, since this exit-code-only
 *                      harness cannot deliver a signal to end a real suspend)
 *
 * The harness is exit-code only (stdout is discarded in standalone terminal
 * mode), so every check returns a distinct non-zero code that pinpoints the
 * failing step; the process exits 0 only when the whole sequence succeeds.
 */

#include <errno.h>
#include <signal.h>
#include <unistd.h>

/* Returns a distinct non-zero code identifying the failing check. */
#define CHECK(step, cond)     \
    do {                      \
        if (!(cond)) {        \
            return (step);    \
        }                     \
    } while (0)

int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    sigset_t set;
    sigset_t old;
    sigset_t cur;

    /*==============================================================================================
     * Signal-set operations.
     *============================================================================================*/

    /* An empty set contains no signals. */
    CHECK(1, sigemptyset(&set) == 0);
    CHECK(2, sigismember(&set, SIGUSR1) == 0);
    CHECK(3, sigismember(&set, SIGUSR2) == 0);

    /* sigaddset marks membership; unrelated signals stay out. */
    CHECK(4, sigaddset(&set, SIGUSR1) == 0);
    CHECK(5, sigismember(&set, SIGUSR1) == 1);
    CHECK(6, sigismember(&set, SIGUSR2) == 0);

    /* sigdelset removes only the requested signal. */
    CHECK(7, sigaddset(&set, SIGUSR2) == 0);
    CHECK(8, sigdelset(&set, SIGUSR1) == 0);
    CHECK(9, sigismember(&set, SIGUSR1) == 0);
    CHECK(10, sigismember(&set, SIGUSR2) == 1);

    /* A full set contains every standard signal. */
    CHECK(11, sigfillset(&set) == 0);
    CHECK(12, sigismember(&set, SIGINT) == 1);
    CHECK(13, sigismember(&set, SIGUSR1) == 1);
    CHECK(14, sigismember(&set, SIGKILL) == 1);

    /* An out-of-range signal number is rejected. */
    CHECK(15, sigismember(&set, 0) == -1);
    CHECK(16, sigaddset(&set, 0) == -1);

    /*==============================================================================================
     * sigprocmask.
     *============================================================================================*/

    /* Start from a known, empty mask. */
    CHECK(17, sigemptyset(&set) == 0);
    CHECK(18, sigprocmask(SIG_SETMASK, &set, NULL) == 0);

    /* Blocking SIGUSR1 reports the previous (empty) mask through oldset. */
    CHECK(19, sigemptyset(&set) == 0);
    CHECK(20, sigaddset(&set, SIGUSR1) == 0);
    CHECK(21, sigprocmask(SIG_BLOCK, &set, &old) == 0);
    CHECK(22, sigismember(&old, SIGUSR1) == 0);

    /* A null `set` leaves the mask unchanged and returns it through oldset. */
    CHECK(23, sigprocmask(SIG_BLOCK, NULL, &cur) == 0);
    CHECK(24, sigismember(&cur, SIGUSR1) == 1);

    /* Unblocking clears only the requested signal. */
    CHECK(25, sigprocmask(SIG_UNBLOCK, &set, NULL) == 0);
    CHECK(26, sigprocmask(SIG_SETMASK, NULL, &cur) == 0);
    CHECK(27, sigismember(&cur, SIGUSR1) == 0);

    /* SIGKILL and SIGSTOP can never be blocked; requests are silently ignored. */
    CHECK(28, sigfillset(&set) == 0);
    CHECK(29, sigprocmask(SIG_SETMASK, &set, NULL) == 0);
    CHECK(30, sigprocmask(SIG_BLOCK, NULL, &cur) == 0);
    CHECK(31, sigismember(&cur, SIGKILL) == 0);
    CHECK(32, sigismember(&cur, SIGSTOP) == 0);

    /* Restore an empty mask. */
    CHECK(33, sigemptyset(&set) == 0);
    CHECK(34, sigprocmask(SIG_SETMASK, &set, NULL) == 0);

    /* An invalid `how` combined with a non-null `set` fails with EINVAL. */
    CHECK(35, sigaddset(&set, SIGUSR1) == 0);
    errno = 0;
    CHECK(36, sigprocmask(-1, &set, NULL) == -1);
    CHECK(37, errno == EINVAL);

    /*==============================================================================================
     * pthread_sigmask.
     *============================================================================================*/

    /* Start from a known, empty mask. */
    CHECK(38, sigemptyset(&set) == 0);
    CHECK(39, pthread_sigmask(SIG_SETMASK, &set, NULL) == 0);

    /* Blocking SIGUSR2 reports the previous (empty) mask through oldset. */
    CHECK(40, sigaddset(&set, SIGUSR2) == 0);
    CHECK(41, pthread_sigmask(SIG_BLOCK, &set, &old) == 0);
    CHECK(42, sigismember(&old, SIGUSR2) == 0);

    /* pthread_sigmask and sigprocmask share the same per-thread mask. */
    CHECK(43, sigprocmask(SIG_BLOCK, NULL, &cur) == 0);
    CHECK(44, sigismember(&cur, SIGUSR2) == 1);

    /* A null `set` returns the current mask through oldset. */
    CHECK(45, pthread_sigmask(SIG_SETMASK, NULL, &cur) == 0);
    CHECK(46, sigismember(&cur, SIGUSR2) == 1);

    /* Unblocking through pthread_sigmask is visible to sigprocmask. */
    CHECK(47, pthread_sigmask(SIG_UNBLOCK, &set, NULL) == 0);
    CHECK(48, sigprocmask(SIG_BLOCK, NULL, &cur) == 0);
    CHECK(49, sigismember(&cur, SIGUSR2) == 0);

    /* SIGKILL and SIGSTOP can never be blocked. */
    CHECK(50, sigfillset(&set) == 0);
    CHECK(51, pthread_sigmask(SIG_SETMASK, &set, NULL) == 0);
    CHECK(52, sigprocmask(SIG_BLOCK, NULL, &cur) == 0);
    CHECK(53, sigismember(&cur, SIGKILL) == 0);
    CHECK(54, sigismember(&cur, SIGSTOP) == 0);

    /* An invalid `how` returns the error number directly instead of -1. */
    CHECK(55, pthread_sigmask(-1, &set, NULL) == EINVAL);

    /* Restore an empty mask. */
    CHECK(56, sigemptyset(&set) == 0);
    CHECK(57, pthread_sigmask(SIG_SETMASK, &set, NULL) == 0);

    /*==============================================================================================
     * sigisemptyset.
     *============================================================================================*/

    /* A freshly emptied set is reported empty; adding a signal makes it non-empty. */
    CHECK(58, sigemptyset(&set) == 0);
    CHECK(59, sigisemptyset(&set) == 1);
    CHECK(60, sigaddset(&set, SIGUSR1) == 0);
    CHECK(61, sigisemptyset(&set) == 0);

    /* Removing the last member empties the set again; a full set is never empty. */
    CHECK(62, sigdelset(&set, SIGUSR1) == 0);
    CHECK(63, sigisemptyset(&set) == 1);
    CHECK(64, sigfillset(&set) == 0);
    CHECK(65, sigisemptyset(&set) == 0);

    /*==============================================================================================
     * Signal-set boundary checks.
     *============================================================================================*/

    /* The largest valid signal number (NSIG - 1) is accepted and round-trips. */
    CHECK(66, sigemptyset(&set) == 0);
    CHECK(67, sigaddset(&set, NSIG - 1) == 0);
    CHECK(68, sigismember(&set, NSIG - 1) == 1);
    CHECK(69, sigdelset(&set, NSIG - 1) == 0);
    CHECK(70, sigismember(&set, NSIG - 1) == 0);

    /* A signal number at or beyond NSIG is out of range and fails with EINVAL. */
    errno = 0;
    CHECK(71, sigaddset(&set, NSIG) == -1);
    CHECK(72, errno == EINVAL);
    errno = 0;
    CHECK(73, sigdelset(&set, NSIG) == -1);
    CHECK(74, errno == EINVAL);
    errno = 0;
    CHECK(75, sigismember(&set, NSIG) == -1);
    CHECK(76, errno == EINVAL);

    /*==============================================================================================
     * Null-pointer handling (EFAULT).
     *============================================================================================*/

    /* Every sigsetops entry point rejects a null set with EFAULT. */
    errno = 0;
    CHECK(77, sigemptyset(NULL) == -1);
    CHECK(78, errno == EFAULT);
    errno = 0;
    CHECK(79, sigfillset(NULL) == -1);
    CHECK(80, errno == EFAULT);
    errno = 0;
    CHECK(81, sigisemptyset(NULL) == -1);
    CHECK(82, errno == EFAULT);
    errno = 0;
    CHECK(83, sigaddset(NULL, SIGUSR1) == -1);
    CHECK(84, errno == EFAULT);
    errno = 0;
    CHECK(85, sigdelset(NULL, SIGUSR1) == -1);
    CHECK(86, errno == EFAULT);
    errno = 0;
    CHECK(87, sigismember(NULL, SIGUSR1) == -1);
    CHECK(88, errno == EFAULT);

    /*==============================================================================================
     * Null-set query no-op.
     *============================================================================================*/

    /* A null `set` together with a null `oldset` is a well-defined no-op that succeeds. */
    CHECK(89, sigprocmask(SIG_SETMASK, NULL, NULL) == 0);
    CHECK(90, pthread_sigmask(SIG_SETMASK, NULL, NULL) == 0);

    /*==============================================================================================
     * sigsuspend argument validation.
     *============================================================================================*/

    /*
     * sigsuspend() suspends until a caught signal arrives, which this exit-code-only
     * harness cannot deliver, so only its argument-validation path is exercised: a
     * null mask is rejected immediately, without suspending, with EINVAL.
     */
    errno = 0;
    CHECK(91, sigsuspend(NULL) == -1);
    CHECK(92, errno == EINVAL);

    /* Success. */
    (void)write(STDOUT_FILENO, "ok", 2);

    return (0);
}
