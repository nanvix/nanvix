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

/*
 * time.h
 * 
 * Struct and function declarations for dealing with time.
 */

#ifndef _TIME_H_
#define _TIME_H_
#ifndef _ASM_FILE_

#include "_ansi.h"
#include <sys/reent.h>

#define __need_size_t
#define __need_NULL
#include <stddef.h>

/* Get _CLOCKS_PER_SEC_ */
#include <machine/time.h>

#ifndef _CLOCKS_PER_SEC_
#define _CLOCKS_PER_SEC_ 1000
#endif

#define CLOCKS_PER_SEC _CLOCKS_PER_SEC_
#define CLK_TCK CLOCKS_PER_SEC

#include <sys/types.h>

_BEGIN_STD_C

struct tm
{
  int	tm_sec;
  int	tm_min;
  int	tm_hour;
  int	tm_mday;
  int	tm_mon;
  int	tm_year;
  int	tm_wday;
  int	tm_yday;
  int	tm_isdst;
#ifdef __TM_GMTOFF
  long	__TM_GMTOFF;
#endif
#ifdef __TM_ZONE
  const char *__TM_ZONE;
#endif
};

clock_t	   _EXFUN(clock,    (void));
double	   _EXFUN(difftime, (time_t _time2, time_t _time1));
time_t	   _EXFUN(mktime,   (struct tm *_timeptr));
time_t	   _EXFUN(time,     (time_t *_timer));
#ifndef _REENT_ONLY
char	  *_EXFUN(asctime,  (const struct tm *_tblock));
char	  *_EXFUN(ctime,    (const time_t *_time));
struct tm *_EXFUN(gmtime,   (const time_t *_timer));
struct tm *_EXFUN(localtime,(const time_t *_timer));
#endif
size_t	   _EXFUN(strftime, (char *__restrict _s,
			     size_t _maxsize, const char *__restrict _fmt,
			     const struct tm *__restrict _t));

char	  *_EXFUN(asctime_r,	(const struct tm *__restrict,
				 char *__restrict));
char	  *_EXFUN(ctime_r,	(const time_t *, char *));
struct tm *_EXFUN(gmtime_r,	(const time_t *__restrict,
				 struct tm *__restrict));
struct tm *_EXFUN(localtime_r,	(const time_t *__restrict,
				 struct tm *__restrict));

_END_STD_C

#ifdef __cplusplus
extern "C" {
#endif

#ifndef __STRICT_ANSI__
char      *_EXFUN(strptime,     (const char *__restrict,
				 const char *__restrict,
				 struct tm *__restrict));
_VOID      _EXFUN(tzset,	(_VOID));
_VOID      _EXFUN(_tzset_r,	(struct _reent *));

typedef struct __tzrule_struct
{
  char ch;
  int m;
  int n;
  int d;
  int s;
  time_t change;
  long offset; /* Match type of _timezone. */
} __tzrule_type;

typedef struct __tzinfo_struct
{
  int __tznorth;
  int __tzyear;
  __tzrule_type __tzrule[2];
} __tzinfo_type;

__tzinfo_type *_EXFUN (__gettzinfo, (_VOID));

/* getdate functions */

#ifdef HAVE_GETDATE
#ifndef _REENT_ONLY
#define getdate_err (*__getdate_err())
int *_EXFUN(__getdate_err,(_VOID));

struct tm *	_EXFUN(getdate, (const char *));
/* getdate_err is set to one of the following values to indicate the error.
     1  the DATEMSK environment variable is null or undefined,
     2  the template file cannot be opened for reading,
     3  failed to get file status information,
     4  the template file is not a regular file,
     5  an error is encountered while reading the template file,
     6  memory allication failed (not enough memory available),
     7  there is no line in the template that matches the input,
     8  invalid input specification  */
#endif /* !_REENT_ONLY */

/* getdate_r returns the error code as above */
int		_EXFUN(getdate_r, (const char *, struct tm *));
#endif /* HAVE_GETDATE */

/* defines for the opengroup specifications Derived from Issue 1 of the SVID.  */
extern __IMPORT long _timezone;
extern __IMPORT int _daylight;
extern __IMPORT char *_tzname[2];

/* POSIX defines the external tzname being defined in time.h */
#ifndef tzname
#define tzname _tzname
#endif
#endif /* !__STRICT_ANSI__ */

#ifdef __cplusplus
}
#endif

#include <sys/features.h>

#ifdef __CYGWIN__
#include <cygwin/time.h>
#endif /*__CYGWIN__*/

#if defined(_POSIX_TIMERS)

#include <signal.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Clocks, P1003.1b-1993, p. 263 */

int _EXFUN(clock_settime, (clockid_t clock_id, const struct timespec *tp));
int _EXFUN(clock_gettime, (clockid_t clock_id, struct timespec *tp));
int _EXFUN(clock_getres,  (clockid_t clock_id, struct timespec *res));

/* Create a Per-Process Timer, P1003.1b-1993, p. 264 */

int _EXFUN(timer_create,
  	(clockid_t clock_id,
 	struct sigevent *__restrict evp,
	timer_t *__restrict timerid));

/* Delete a Per_process Timer, P1003.1b-1993, p. 266 */

int _EXFUN(timer_delete, (timer_t timerid));

/* Per-Process Timers, P1003.1b-1993, p. 267 */

int _EXFUN(timer_settime,
	(timer_t timerid, int flags,
	const struct itimerspec *__restrict value,
	struct itimerspec *__restrict ovalue));
int _EXFUN(timer_gettime, (timer_t timerid, struct itimerspec *value));
int _EXFUN(timer_getoverrun, (timer_t timerid));

/* High Resolution Sleep, P1003.1b-1993, p. 269 */

int _EXFUN(nanosleep, (const struct timespec  *rqtp, struct timespec *rmtp));

#ifdef __cplusplus
}
#endif
#endif /* _POSIX_TIMERS */

#if defined(_POSIX_CLOCK_SELECTION)

#ifdef __cplusplus
extern "C" {
#endif

int _EXFUN(clock_nanosleep,
  (clockid_t clock_id, int flags, const struct timespec *rqtp,
   struct timespec *rmtp));

#ifdef __cplusplus
}
#endif

#endif /* _POSIX_CLOCK_SELECTION */

#ifdef __cplusplus
extern "C" {
#endif

/* CPU-time Clock Attributes, P1003.4b/D8, p. 54 */

/* values for the clock enable attribute */

#define CLOCK_ENABLED  1  /* clock is enabled, i.e. counting execution time */
#define CLOCK_DISABLED 0  /* clock is disabled */

/* values for the pthread cputime_clock_allowed attribute */

#define CLOCK_ALLOWED    1 /* If a thread is created with this value a */
                           /*   CPU-time clock attached to that thread */
                           /*   shall be accessible. */
#define CLOCK_DISALLOWED 0 /* If a thread is created with this value, the */
                           /*   thread shall not have a CPU-time clock */
                           /*   accessible. */

/* Manifest Constants, P1003.1b-1993, p. 262 */

#define CLOCK_REALTIME (clockid_t)1

/* Flag indicating time is "absolute" with respect to the clock
   associated with a time.  */

#define TIMER_ABSTIME	4

/* Manifest Constants, P1003.4b/D8, p. 55 */

#if defined(_POSIX_CPUTIME)

/* When used in a clock or timer function call, this is interpreted as
   the identifier of the CPU_time clock associated with the PROCESS
   making the function call.  */

#define CLOCK_PROCESS_CPUTIME_ID (clockid_t)2

#endif

#if defined(_POSIX_THREAD_CPUTIME)

/*  When used in a clock or timer function call, this is interpreted as
    the identifier of the CPU_time clock associated with the THREAD
    making the function call.  */

#define CLOCK_THREAD_CPUTIME_ID (clockid_t)3

#endif

#if defined(_POSIX_MONOTONIC_CLOCK)

/*  The identifier for the system-wide monotonic clock, which is defined
 *      as a clock whose value cannot be set via clock_settime() and which 
 *          cannot have backward clock jumps. */

#define CLOCK_MONOTONIC (clockid_t)4

#endif

#if defined(_POSIX_CPUTIME)

/* Accessing a Process CPU-time CLock, P1003.4b/D8, p. 55 */

int _EXFUN(clock_getcpuclockid, (pid_t pid, clockid_t *clock_id));

#endif /* _POSIX_CPUTIME */

#if defined(_POSIX_CPUTIME) || defined(_POSIX_THREAD_CPUTIME)

/* CPU-time Clock Attribute Access, P1003.4b/D8, p. 56 */

int _EXFUN(clock_setenable_attr, (clockid_t clock_id, int attr));
int _EXFUN(clock_getenable_attr, (clockid_t clock_id, int *attr));

#endif /* _POSIX_CPUTIME or _POSIX_THREAD_CPUTIME */

#ifdef __cplusplus
}
#endif

#endif /* _ASM_FILE_ */
#endif /* _TIME_H_ */
