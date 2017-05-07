/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

/* sys/signal.h */

#ifndef _SYS_SIGNAL_H
#define _SYS_SIGNAL_H
#ifdef __cplusplus
extern "C" {
#endif

#include "_ansi.h"
#include <sys/features.h>
#include <sys/types.h>

/* #ifndef __STRICT_ANSI__*/

/* Cygwin defines it's own sigset_t in include/cygwin/signal.h */
#ifndef __CYGWIN__
typedef unsigned long sigset_t;
#endif

#if defined(__rtems__)

#if defined(_POSIX_REALTIME_SIGNALS)

/* sigev_notify values
   NOTE: P1003.1c/D10, p. 34 adds SIGEV_THREAD.  */

#define SIGEV_NONE   1  /* No asynchronous notification shall be delivered */
                        /*   when the event of interest occurs. */
#define SIGEV_SIGNAL 2  /* A queued signal, with an application defined */
                        /*  value, shall be delivered when the event of */
                        /*  interest occurs. */
#define SIGEV_THREAD 3  /* A notification function shall be called to */
                        /*   perform notification. */

/*  Signal Generation and Delivery, P1003.1b-1993, p. 63
    NOTE: P1003.1c/D10, p. 34 adds sigev_notify_function and
          sigev_notify_attributes to the sigevent structure.  */

union sigval {
  int    sival_int;    /* Integer signal value */
  void  *sival_ptr;    /* Pointer signal value */
};

struct sigevent {
  int              sigev_notify;               /* Notification type */
  int              sigev_signo;                /* Signal number */
  union sigval     sigev_value;                /* Signal value */

#if defined(_POSIX_THREADS)
  void           (*sigev_notify_function)( union sigval );
                                               /* Notification function */
  pthread_attr_t  *sigev_notify_attributes;    /* Notification Attributes */
#endif
};

/* Signal Actions, P1003.1b-1993, p. 64 */
/* si_code values, p. 66 */

#define SI_USER    1    /* Sent by a user. kill(), abort(), etc */
#define SI_QUEUE   2    /* Sent by sigqueue() */
#define SI_TIMER   3    /* Sent by expiration of a timer_settime() timer */
#define SI_ASYNCIO 4    /* Indicates completion of asycnhronous IO */
#define SI_MESGQ   5    /* Indicates arrival of a message at an empty queue */

typedef struct {
  int          si_signo;    /* Signal number */
  int          si_code;     /* Cause of the signal */
  union sigval si_value;    /* Signal value */
} siginfo_t;
#endif

/*  3.3.8 Synchronously Accept a Signal, P1003.1b-1993, p. 76 */

#define SA_NOCLDSTOP 0x1   /* Do not generate SIGCHLD when children stop */
#define SA_SIGINFO   0x2   /* Invoke the signal catching function with */
                           /*   three arguments instead of one. */
#if __BSD_VISIBLE || __XSI_VISIBLE || __POSIX_VISIBLE >= 200112
#define SA_ONSTACK   0x4   /* Signal delivery will be on a separate stack. */
#endif

/* struct sigaction notes from POSIX:
 *
 *  (1) Routines stored in sa_handler should take a single int as
 *      their argument although the POSIX standard does not require this.
 *      This is not longer true since at least POSIX.1-2008
 *  (2) The fields sa_handler and sa_sigaction may overlap, and a conforming
 *      application should not use both simultaneously.
 */

typedef void (*_sig_func_ptr)(int);

struct sigaction {
  int         sa_flags;       /* Special flags to affect behavior of signal */
  sigset_t    sa_mask;        /* Additional set of signals to be blocked */
                              /*   during execution of signal-catching */
                              /*   function. */
  union {
    _sig_func_ptr _handler;  /* SIG_DFL, SIG_IGN, or pointer to a function */
#if defined(_POSIX_REALTIME_SIGNALS)
    void      (*_sigaction)( int, siginfo_t *, void * );
#endif
  } _signal_handlers;
};

#define sa_handler    _signal_handlers._handler
#if defined(_POSIX_REALTIME_SIGNALS)
#define sa_sigaction  _signal_handlers._sigaction
#endif

#if __BSD_VISIBLE || __XSI_VISIBLE || __POSIX_VISIBLE >= 200112
/*
 * Minimum and default signal stack constants. Allow for target overrides
 * from <sys/features.h>.
 */
#ifndef	MINSIGSTKSZ
#define	MINSIGSTKSZ	2048
#endif
#ifndef	SIGSTKSZ
#define	SIGSTKSZ	8192
#endif

/*
 * Possible values for ss_flags in stack_t below.
 */
#define	SS_ONSTACK	0x1
#define	SS_DISABLE	0x2

/*
 * Structure used in sigaltstack call.
 */
typedef struct sigaltstack {
  void     *ss_sp;    /* Stack base or pointer.  */
  int       ss_flags; /* Flags.  */
  size_t    ss_size;  /* Stack size.  */
} stack_t;
#endif

#elif defined(__CYGWIN__)
#include <cygwin/signal.h>
#else
#define SA_NOCLDSTOP 1  /* only value supported now for sa_flags */

typedef void (*_sig_func_ptr)(int);

struct sigaction 
{
	_sig_func_ptr sa_handler;
	sigset_t sa_mask;
	int sa_flags;
};
#endif /* defined(__rtems__) */

#define SIG_SETMASK 0	/* set mask with sigprocmask() */
#define SIG_BLOCK 1	/* set of signals to block */
#define SIG_UNBLOCK 2	/* set of signals to, well, unblock */

/* These depend upon the type of sigset_t, which right now 
   is always a long.. They're in the POSIX namespace, but
   are not ANSI. */
#define sigaddset(what,sig) (*(what) |= (1<<(sig)), 0)
#define sigdelset(what,sig) (*(what) &= ~(1<<(sig)), 0)
#define sigemptyset(what)   (*(what) = 0, 0)
#define sigfillset(what)    (*(what) = ~(0), 0)
#define sigismember(what,sig) (((*(what)) & (1<<(sig))) != 0)

int _EXFUN(sigprocmask, (int how, const sigset_t *set, sigset_t *oset));

#if defined(_POSIX_THREADS)
int _EXFUN(pthread_sigmask, (int how, const sigset_t *set, sigset_t *oset));
#endif

#if defined(__CYGWIN__) || defined(__rtems__)
#undef sigaddset
#undef sigdelset
#undef sigemptyset
#undef sigfillset
#undef sigismember

#ifdef _COMPILING_NEWLIB
int _EXFUN(_kill, (pid_t, int));
#endif /* _COMPILING_NEWLIB */
#endif /* __CYGWIN__ || __rtems__ */
int _EXFUN(kill, (pid_t, int));
#if defined(__CYGWIN__) || defined(__rtems__)
int _EXFUN(killpg, (pid_t, int));
int _EXFUN(sigaction, (int, const struct sigaction *, struct sigaction *));
int _EXFUN(sigaddset, (sigset_t *, const int));
int _EXFUN(sigdelset, (sigset_t *, const int));
int _EXFUN(sigismember, (const sigset_t *, int));
int _EXFUN(sigfillset, (sigset_t *));
int _EXFUN(sigemptyset, (sigset_t *));
int _EXFUN(sigpending, (sigset_t *));
int _EXFUN(sigsuspend, (const sigset_t *));
int _EXFUN(sigpause, (int));

#ifdef __rtems__
#if __BSD_VISIBLE || __XSI_VISIBLE || __POSIX_VISIBLE >= 200112
int _EXFUN(sigaltstack, (const stack_t *__restrict, stack_t *__restrict));
#endif
#endif

#if defined(_POSIX_THREADS)
#ifdef __CYGWIN__
#  ifndef _CYGWIN_TYPES_H
#    error You need the winsup sources or a cygwin installation to compile the cygwin version of newlib.
#  endif
#endif
int _EXFUN(pthread_kill, (pthread_t thread, int sig));
#endif

#if defined(_POSIX_REALTIME_SIGNALS)

/*  3.3.8 Synchronously Accept a Signal, P1003.1b-1993, p. 76
    NOTE: P1003.1c/D10, p. 39 adds sigwait().  */

int _EXFUN(sigwaitinfo, (const sigset_t *set, siginfo_t *info));
int _EXFUN(sigtimedwait,
  (const sigset_t *set, siginfo_t *info, const struct timespec  *timeout)
);
int _EXFUN(sigwait, (const sigset_t *set, int *sig));

/*  3.3.9 Queue a Signal to a Process, P1003.1b-1993, p. 78 */
int _EXFUN(sigqueue, (pid_t pid, int signo, const union sigval value));

#endif /* defined(_POSIX_REALTIME_SIGNALS) */

#endif /* defined(__CYGWIN__) || defined(__rtems__) */

/* #endif __STRICT_ANSI__ */
#ifdef __cplusplus
}
#endif

#ifndef _SIGNAL_H_
/* Some applications take advantage of the fact that <sys/signal.h>
 * and <signal.h> are equivalent in glibc.  Allow for that here.  */
#include <signal.h>
#endif
#endif /* _SYS_SIGNAL_H */
