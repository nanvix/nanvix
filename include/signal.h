/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_SIGNAL_H
#define _NANVIX_SIGNAL_H

/**
 * @file signal.h
 * @brief Signal handling.
 *
 * Declares functions and types for signal delivery and management.
 * Implemented by the libc_signal Rust crate.
 */

#include <stdint.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/** @brief Signal handler function pointer. */
typedef void (*sighandler_t)(int);

/** @brief Signal set type. */
typedef uint64_t sigset_t;

/** @brief Type that can be accessed atomically with respect to signals. */
typedef int sig_atomic_t;

/** @brief Value that accompanies a signal (used by SA_SIGINFO handlers). */
union sigval {
    int sival_int;   /**< Integer signal value. */
    void *sival_ptr; /**< Pointer signal value. */
};

/**
 * @brief Information passed to an SA_SIGINFO signal handler.
 *
 * Minimal POSIX subset: only the members common to every signal are provided.
 * The guest does not yet populate this structure because asynchronous signal
 * delivery is not implemented.
 */
typedef struct {
    int si_signo;          /**< Signal number.                        */
    int si_code;           /**< Signal code (one of the SI_* values). */
    int si_errno;          /**< errno value associated with signal.   */
    union sigval si_value; /**< Signal value.                         */
} siginfo_t;

/** @brief Structure for sigaction(). */
struct sigaction {
    sighandler_t sa_handler; /**< Signal handler.              */
    sigset_t sa_mask;        /**< Signals to block in handler. */
    int sa_flags;            /**< Action flags.                */
    void (*sa_sigaction)(int, siginfo_t *, void *); /**< Handler used with SA_SIGINFO. */
};

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#define SIG_DFL ((sighandler_t)0)
#define SIG_IGN ((sighandler_t)1)
#define SIG_ERR ((sighandler_t) - 1)

/* `how` argument values for sigprocmask(). */
#define SIG_BLOCK 0
#define SIG_UNBLOCK 1
#define SIG_SETMASK 2

/* Flags for sigaction(). */
#define SA_NOCLDSTOP 0x00000001
#define SA_NOCLDWAIT 0x00000002
#define SA_SIGINFO 0x00000004
#define SA_ONSTACK 0x08000000
#define SA_RESTART 0x10000000
#define SA_NODEFER 0x40000000
#define SA_RESETHAND 0x80000000

/* `si_code` values carried by siginfo_t. */
#define SI_USER 0
#define SI_QUEUE (-1)
#define SI_TIMER (-2)
#define SI_MESGQ (-3)
#define SI_ASYNCIO (-4)

/*==================================================================================================
 * Signal Numbers
 *==================================================================================================*/

/* Standard signal numbers. */
#ifndef SIGHUP
#define SIGHUP 1
#define SIGINT 2
#define SIGQUIT 3
#define SIGILL 4
#define SIGTRAP 5
#define SIGABRT 6
#define SIGBUS 7
#define SIGFPE 8
#define SIGKILL 9
#define SIGUSR1 10
#define SIGSEGV 11
#define SIGUSR2 12
#define SIGPIPE 13
#define SIGALRM 14
#define SIGTERM 15
#define SIGCHLD 17
#define SIGCONT 18
#define SIGSTOP 19
#define SIGTSTP 20
#define SIGTTIN 21
#define SIGTTOU 22
#define SIGURG 23
#define SIGXCPU 24
#define SIGXFSZ 25
#define SIGVTALRM 26
#define SIGPROF 27
#define SIGWINCH 28
#define SIGIO 29
#define SIGSYS 31
#define _NSIG 65
#endif

/*==================================================================================================
 * Signal Functions
 *==================================================================================================*/

extern sighandler_t signal(int signum, sighandler_t handler);
extern int raise(int sig);
extern int kill(pid_t pid, int sig);

/*==================================================================================================
 * Signal Set Functions
 *==================================================================================================*/

extern int sigemptyset(sigset_t *set);
extern int sigfillset(sigset_t *set);
extern int sigaddset(sigset_t *set, int signum);
extern int sigdelset(sigset_t *set, int signum);
extern int sigismember(const sigset_t *set, int signum);
extern int sigprocmask(int how, const sigset_t *set, sigset_t *oldset);
extern int sigsuspend(const sigset_t *mask);
extern int sigaction(int signum, const struct sigaction *act, struct sigaction *oldact);
extern int pthread_sigmask(int how, const sigset_t *set, sigset_t *oldset);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_SIGNAL_H */
