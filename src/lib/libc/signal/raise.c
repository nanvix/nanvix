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

/* Embedded systems may want the simulated signals if no other form exists,
   but UNIX versions will want to use the host facilities.
   Define SIMULATED_SIGNALS when you want to use the simulated versions.
*/

/*
FUNCTION
<<raise>>---send a signal

INDEX
	raise
INDEX
	_raise_r

ANSI_SYNOPSIS
	#include <signal.h>
	int raise(int <[sig]>);

	int _raise_r(void *<[reent]>, int <[sig]>);

TRAD_SYNOPSIS
	#include <signal.h>
	int raise(<[sig]>)
	int <[sig]>;

	int _raise_r(<[reent]>, <[sig]>)
	char *<[reent]>;
	int <[sig]>;

DESCRIPTION
Send the signal <[sig]> (one of the macros from `<<sys/signal.h>>').
This interrupts your program's normal flow of execution, and allows a signal
handler (if you've defined one, using <<signal>>) to take control.

The alternate function <<_raise_r>> is a reentrant version.  The extra
argument <[reent]> is a pointer to a reentrancy structure.

RETURNS
The result is <<0>> if <[sig]> was successfully raised, <<1>>
otherwise.  However, the return value (since it depends on the normal
flow of execution) may not be visible, unless the signal handler for
<[sig]> terminates with a <<return>> or unless <<SIG_IGN>> is in
effect for this signal.

PORTABILITY
ANSI C requires <<raise>>, but allows the full set of signal numbers
to vary from one implementation to another.

Required OS subroutines: <<getpid>>, <<kill>>.
*/

#ifndef SIGNAL_PROVIDED

int _dummy_raise;

#else

#include <reent.h>
#include <signal.h>

#ifndef _REENT_ONLY

int
_DEFUN (raise, (sig),
	int sig)
{
  return _raise_r (_REENT, sig);
}

#endif

int
_DEFUN (_raise_r, (reent, sig),
	struct _reent *reent _AND
	int sig)
{
  return _kill_r (reent, _getpid_r (reent), sig);
}

#endif /* SIGNAL_PROVIDED */
