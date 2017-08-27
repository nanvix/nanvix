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
 * COmmon routine to call call registered atexit-like routines.
 */


#include <stdlib.h>
#include <reent.h>
#include <sys/lock.h>
#include "atexit.h"

#ifdef __GNUC__
#define UNUSED_VAR __attribute__ ((unused))
#else
#define UNUSED_VAR
#endif

UNUSED_VAR __LOCK_INIT_RECURSIVE(, __atexit_lock)

#ifdef _REENT_GLOBAL_ATEXIT
struct _atexit *_global_atexit = _NULL;
#endif

#ifdef _WANT_REGISTER_FINI

/* If "__libc_fini" is defined, finalizers (either
   "__libc_fini_array", or "_fini", as appropriate) will be run after
   all user-specified atexit handlers.  For example, you can define
   "__libc_fini" to "_fini" in your linker script if you want the C
   library, rather than startup code, to register finalizers.  If you
   do that, then your startup code need not contain references to
   "atexit" or "exit".  As a result, only applications that reference
   "exit" explicitly will pull in finalization code.

   The choice of whether to register finalizers from libc or from
   startup code is deferred to link-time, rather than being a
   configure-time option, so that the same C library binary can be
   used with multiple BSPs, some of which register finalizers from
   startup code, while others defer to the C library.  */
extern char __libc_fini __attribute__((weak));

/* Register the application finalization function with atexit.  These
   finalizers should run last.  Therefore, we want to call atexit as
   soon as possible.  */
static void 
register_fini(void) __attribute__((constructor (0)));

static void 
register_fini(void)
{
  if (&__libc_fini) {
#ifdef HAVE_INITFINI_ARRAY
    extern void __libc_fini_array (void);
    atexit (__libc_fini_array);
#else
    extern void _fini (void);
    atexit (_fini);
#endif
  }
}

#endif /* _WANT_REGISTER_FINI  */

/*
 * Call registered exit handlers.  If D is null then all handlers are called,
 * otherwise only the handlers from that DSO are called.
 */

void 
_DEFUN (__call_exitprocs, (code, d),
	int code _AND _PTR d)
{
  register struct _atexit *p;
  struct _atexit **lastp;
  register struct _on_exit_args * args;
  register int n;
  int i;
  void (*fn) (void);


#ifndef __SINGLE_THREAD__
  __lock_acquire_recursive(__atexit_lock);
#endif

 restart:

  p = _GLOBAL_ATEXIT;
  lastp = &_GLOBAL_ATEXIT;
  while (p)
    {
#ifdef _REENT_SMALL
      args = p->_on_exit_args_ptr;
#else
      args = &p->_on_exit_args;
#endif
      for (n = p->_ind - 1; n >= 0; n--)
	{
	  int ind;

	  i = 1 << n;

	  /* Skip functions not from this dso.  */
	  if (d && (!args || args->_dso_handle[n] != d))
	    continue;

	  /* Remove the function now to protect against the
	     function calling exit recursively.  */
	  fn = p->_fns[n];
	  if (n == p->_ind - 1)
	    p->_ind--;
	  else
	    p->_fns[n] = NULL;

	  /* Skip functions that have already been called.  */
	  if (!fn)
	    continue;

	  ind = p->_ind;

	  /* Call the function.  */
	  if (!args || (args->_fntypes & i) == 0)
	    fn ();
	  else if ((args->_is_cxa & i) == 0)
	    (*((void (*)(int, _PTR)) fn))(code, args->_fnargs[n]);
	  else
	    (*((void (*)(_PTR)) fn))(args->_fnargs[n]);

	  /* The function we called call atexit and registered another
	     function (or functions).  Call these new functions before
	     continuing with the already registered functions.  */
	  if (ind != p->_ind || *lastp != p)
	    goto restart;
	}

#ifndef _ATEXIT_DYNAMIC_ALLOC
      break;
#else
      /* Don't dynamically free the atexit array if free is not
	 available.  */
      if (!free)
	break;

      /* Move to the next block.  Free empty blocks except the last one,
	 which is part of _GLOBAL_REENT.  */
      if (p->_ind == 0 && p->_next)
	{
	  /* Remove empty block from the list.  */
	  *lastp = p->_next;
#ifdef _REENT_SMALL
	  if (args)
	    free (args);
#endif
	  free (p);
	  p = *lastp;
	}
      else
	{
	  lastp = &p->_next;
	  p = p->_next;
	}
#endif
    }
#ifndef __SINGLE_THREAD__
  __lock_release_recursive(__atexit_lock);
#endif

}
