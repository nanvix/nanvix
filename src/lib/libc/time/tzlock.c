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
FUNCTION
<<__tz_lock>>, <<__tz_unlock>>---lock time zone global variables

INDEX
	__tz_lock
INDEX
	__tz_unlock

ANSI_SYNOPSIS
	#include "local.h"
	void __tz_lock (void);
	void __tz_unlock (void);

TRAD_SYNOPSIS
	void __tz_lock();
	void __tz_unlock();

DESCRIPTION
The <<tzset>> facility functions call these functions when they need to
ensure the values of global variables.  The version of these routines
supplied in the library use the lock API defined in sys/lock.h.  If multiple
threads of execution can call the time functions and give up scheduling in
the middle, then you you need to define your own versions of these functions
in order to safely lock the time zone variables during a call.  If you do
not, the results of <<localtime>>, <<mktime>>, <<ctime>>, and <<strftime>>
are undefined.

The lock <<__tz_lock>> may not be called recursively; that is,
a call <<__tz_lock>> will always lock all subsequent <<__tz_lock>> calls
until the corresponding <<__tz_unlock>> call on the same thread is made.
*/

#include <_ansi.h>
#include "local.h"
#include <sys/lock.h>

#ifdef __GNUC__
#define UNUSED_VAR __attribute__ ((unused))
#else
#define UNUSED_VAR
#endif

#ifndef __SINGLE_THREAD__
UNUSED_VAR __LOCK_INIT(static, __tz_lock_object)
#endif

_VOID
_DEFUN_VOID (__tz_lock)
{
#ifndef __SINGLE_THREAD__
  __lock_acquire(__tz_lock_object);
#endif
}

_VOID
_DEFUN_VOID (__tz_unlock)
{
#ifndef __SINGLE_THREAD__
  __lock_release(__tz_lock_object);
#endif
}
