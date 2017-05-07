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
	<<reent>>---definition of impure data.
	
INDEX
	reent

DESCRIPTION
	This module defines the impure data area used by the
	non-reentrant functions, such as strtok.
*/

#include <stdlib.h>
#include <reent.h>

#ifdef _REENT_ONLY
#ifndef REENTRANT_SYSCALLS_PROVIDED
#define REENTRANT_SYSCALLS_PROVIDED
#endif
#endif


/* Interim cleanup code */

void
_DEFUN (cleanup_glue, (ptr, glue),
     struct _reent *ptr _AND
     struct _glue *glue)
{
  /* Have to reclaim these in reverse order: */
  if (glue->_next)
    cleanup_glue (ptr, glue->_next);

  _free_r (ptr, glue);
}

void
_DEFUN (_reclaim_reent, (ptr),
     struct _reent *ptr)
{
  if (ptr != _impure_ptr)
    {
      /* used by mprec routines. */
#ifdef _REENT_SMALL
      if (ptr->_mp)	/* don't bother allocating it! */
      {
#endif
      if (_REENT_MP_FREELIST(ptr))
	{
	  int i;
	  for (i = 0; i < (int)_Kmax; i++) 
	    {
	      struct _Bigint *thisone, *nextone;
	
	      nextone = _REENT_MP_FREELIST(ptr)[i];
	      while (nextone)
		{
		  thisone = nextone;
		  nextone = nextone->_next;
		  _free_r (ptr, thisone);
		}
	    }    

	  _free_r (ptr, _REENT_MP_FREELIST(ptr));
	}
      if (_REENT_MP_RESULT(ptr))
	_free_r (ptr, _REENT_MP_RESULT(ptr));
#ifdef _REENT_SMALL
      }
#endif

#ifdef _REENT_SMALL
      if (ptr->_emergency)
	_free_r (ptr, ptr->_emergency);
      if (ptr->_mp)
	_free_r (ptr, ptr->_mp);
      if (ptr->_r48)
	_free_r (ptr, ptr->_r48);
      if (ptr->_localtime_buf)
	_free_r (ptr, ptr->_localtime_buf);
      if (ptr->_asctime_buf)
	_free_r (ptr, ptr->_asctime_buf);
	  if (ptr->_signal_buf)
	_free_r (ptr, ptr->_signal_buf);
	  if (ptr->_misc)
	_free_r (ptr, ptr->_misc);
#endif

#ifndef _REENT_GLOBAL_ATEXIT
      /* atexit stuff */
# ifdef _REENT_SMALL
      if (ptr->_atexit && ptr->_atexit->_on_exit_args_ptr)
	_free_r (ptr, ptr->_atexit->_on_exit_args_ptr);
# else
      if ((ptr->_atexit) && (ptr->_atexit != &ptr->_atexit0))
	{
	  struct _atexit *p, *q;
	  for (p = ptr->_atexit; p != &ptr->_atexit0;)
	    {
	      q = p;
	      p = p->_next;
	      _free_r (ptr, q);
	    }
	}
# endif
#endif

      if (ptr->_cvtbuf)
	_free_r (ptr, ptr->_cvtbuf);
    /* We should free _sig_func to avoid a memory leak, but how to
	   do it safely considering that a signal may be delivered immediately
	   after the free?
	  if (ptr->_sig_func)
	_free_r (ptr, ptr->_sig_func);*/

      if (ptr->__sdidinit)
	{
	  /* cleanup won't reclaim memory 'coz usually it's run
	     before the program exits, and who wants to wait for that? */
	  ptr->__cleanup (ptr);

	  if (ptr->__sglue._next)
	    cleanup_glue (ptr, ptr->__sglue._next);
	}

      /* Malloc memory not reclaimed; no good way to return memory anyway. */

    }
}
