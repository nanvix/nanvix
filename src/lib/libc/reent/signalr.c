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

/* Reentrant versions of syscalls need to support signal/raise.
   These implementations just call the usual system calls.  */

#include <reent.h>
#include <signal.h>
#include <unistd.h>
#include <_syslist.h>

/* Some targets provides their own versions of these functions.  Those
   targets should define REENTRANT_SYSCALLS_PROVIDED in TARGET_CFLAGS.  */

#ifdef _REENT_ONLY
#ifndef REENTRANT_SYSCALLS_PROVIDED
#define REENTRANT_SYSCALLS_PROVIDED
#endif
#endif

#ifdef REENTRANT_SYSCALLS_PROVIDED

int _dummy_link_syscalls = 1;

#else

/* We use the errno variable used by the system dependent layer.  */
#undef errno
extern int errno;

/*
FUNCTION
	<<_kill_r>>---Reentrant version of kill
	
INDEX
	_kill_r

ANSI_SYNOPSIS
	#include <reent.h>
	int _kill_r(struct _reent *<[ptr]>, int <[pid]>, int <[sig]>);

TRAD_SYNOPSIS
	#include <reent.h>
	int _kill_r(<[ptr]>, <[pid]>, <[sig]>)
	struct _reent *<[ptr]>;
	int <[pid]>;
	int <[sig]>;

DESCRIPTION
	This is a reentrant version of <<kill>>.  It
	takes a pointer to the global data block, which holds
	<<errno>>.
*/

int
_DEFUN (_kill_r, (ptr, pid, sig),
     struct _reent *ptr _AND
     int pid _AND
     int sig)
{
  ((void)ptr);	
  int ret;

  errno = 0;
  if ((ret = _kill (pid, sig)) == -1 && errno != 0)
    ptr->_errno = errno;
  return ret;
}

/*
FUNCTION
	<<_getpid_r>>---Reentrant version of getpid
	
INDEX
	_getpid_r

ANSI_SYNOPSIS
	#include <reent.h>
	int _getpid_r(struct _reent *<[ptr]>);

TRAD_SYNOPSIS
	#include <reent.h>
	int _getpid_r(<[ptr]>)
	struct _reent *<[ptr]>;

DESCRIPTION
	This is a reentrant version of <<getpid>>.  It
	takes a pointer to the global data block, which holds
	<<errno>>.

	We never need <<errno>>, of course, but for consistency we
	still must have the reentrant pointer argument.
*/

int
_DEFUN (_getpid_r, (ptr),
     struct _reent *ptr)
{
  ((void)ptr);	
  int ret;
  ret = _getpid ();
  return ret;
}

#endif /* ! defined (REENTRANT_SYSCALLS_PROVIDED) */
