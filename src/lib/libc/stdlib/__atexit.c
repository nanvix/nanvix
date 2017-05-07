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
 *  Common routine to implement atexit-like functionality.
 *
 *  This is also the key function to be configured as lite exit, a size-reduced
 *  implementation of exit that doesn't invoke clean-up functions such as _fini
 *  or global destructors.
 *
 *  Default (without lite exit) call graph is like:
 *  _start -> atexit -> __register_exitproc
 *  _start -> __libc_init_array -> __cxa_atexit -> __register_exitproc
 *  on_exit -> __register_exitproc
 *  _start -> exit -> __call_exitprocs
 *
 *  Here an -> means arrow tail invokes arrow head. All invocations here
 *  are non-weak reference in current newlib/libgloss.
 *
 *  Lite exit makes some of above calls as weak reference, so that size expansive
 *  functions __register_exitproc and __call_exitprocs may not be linked. These
 *  calls are:
 *    _start w-> atexit
 *    __cxa_atexit w-> __register_exitproc
 *    exit w-> __call_exitprocs
 *
 *  Lite exit also makes sure that __call_exitprocs will be referenced as non-weak
 *  whenever __register_exitproc is referenced as non-weak.
 *
 *  Thus with lite exit libs, a program not explicitly calling atexit or on_exit
 *  will escape from the burden of cleaning up code. A program with atexit or on_exit
 *  will work consistently to normal libs.
 *
 *  Lite exit is enabled with --enable-lite-exit, and is controlled with macro
 *  _LITE_EXIT.
 */

#include <stddef.h>
#include <stdlib.h>
#include <reent.h>
#include <sys/lock.h>
#include "atexit.h"


#ifdef _LITE_EXIT
/* As __call_exitprocs is weak reference in lite exit, make a
   non-weak reference to it here.  */
const void * __atexit_dummy = &__call_exitprocs;
#endif

#ifndef __SINGLE_THREAD__
extern _LOCK_RECURSIVE_T __atexit_lock;
#endif

#ifdef _REENT_GLOBAL_ATEXIT
static struct _atexit _global_atexit0 = _ATEXIT_INIT;
# define _GLOBAL_ATEXIT0 (&_global_atexit0)
#else
# define _GLOBAL_ATEXIT0 (&_GLOBAL_REENT->_atexit0)
#endif

/*
 * Register a function to be performed at exit or on shared library unload.
 */

int
_DEFUN (__register_exitproc,
	(type, fn, arg, d),
	int type _AND
	void (*fn) (void) _AND
	void *arg _AND
	void *d)
{
  struct _on_exit_args * args;
  register struct _atexit *p;

#ifndef __SINGLE_THREAD__
  __lock_acquire_recursive(__atexit_lock);
#endif

  p = _GLOBAL_ATEXIT;
  if (p == NULL)
    _GLOBAL_ATEXIT = p = _GLOBAL_ATEXIT0;
  if (p->_ind >= _ATEXIT_SIZE)
    {
#ifndef _ATEXIT_DYNAMIC_ALLOC
      return -1;
#else
      /* Don't dynamically allocate the atexit array if malloc is not
	 available.  */
      if (!malloc)
	return -1;

      p = (struct _atexit *) malloc (sizeof *p);
      if (p == NULL)
	{
#ifndef __SINGLE_THREAD__
	  __lock_release_recursive(__atexit_lock);
#endif
	  return -1;
	}
      p->_ind = 0;
      p->_next = _GLOBAL_ATEXIT;
      _GLOBAL_ATEXIT = p;
#ifndef _REENT_SMALL
      p->_on_exit_args._fntypes = 0;
      p->_on_exit_args._is_cxa = 0;
#else
      p->_on_exit_args_ptr = NULL;
#endif
#endif
    }

  if (type != __et_atexit)
    {
#ifdef _REENT_SMALL
      args = p->_on_exit_args_ptr;
      if (args == NULL)
	{
	  if (malloc)
	    args = malloc (sizeof * p->_on_exit_args_ptr);

	  if (args == NULL)
	    {
#ifndef __SINGLE_THREAD__
	      __lock_release(__atexit_lock);
#endif
	      return -1;
	    }
	  args->_fntypes = 0;
	  args->_is_cxa = 0;
	  p->_on_exit_args_ptr = args;
	}
#else
      args = &p->_on_exit_args;
#endif
      args->_fnargs[p->_ind] = arg;
      args->_fntypes |= (1 << p->_ind);
      args->_dso_handle[p->_ind] = d;
      if (type == __et_cxa)
	args->_is_cxa |= (1 << p->_ind);
    }
  p->_fns[p->_ind++] = fn;
#ifndef __SINGLE_THREAD__
  __lock_release_recursive(__atexit_lock);
#endif
  return 0;
}
