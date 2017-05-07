/*
 * Copyright(C) 2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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
 * Copyright (c) 1990 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms are permitted
 * provided that the above copyright notice and this paragraph are
 * duplicated in all such forms and that any documentation,
 * advertising materials, and other materials related to such
 * distribution and use acknowledge that the software was developed
 * by the University of California, Berkeley.  The name of the
 * University may not be used to endorse or promote products derived
 * from this software without specific prior written permission.
 * THIS SOFTWARE IS PROVIDED ``AS IS'' AND WITHOUT ANY EXPRESS OR
 * IMPLIED WARRANTIES, INCLUDING, WITHOUT LIMITATION, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE.
 */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <string.h>
#include <time.h>
#include <fcntl.h>
#include <stdlib.h>
#include <errno.h>
#include <sys/stat.h>
#include "local.h"

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

#define	POS_ERR	(-(_fpos_t)1)

int _fseeko_r(struct _reent *ptr, register FILE *fp, _off_t offset, int whence)
{
  _fpos_t _EXFNPTR(seekfn, (struct _reent *, _PTR, _fpos_t, int));
  _fpos_t target;
  _fpos_t curoff = 0;
  size_t n;
#ifdef __USE_INTERNAL_STAT64
  struct stat64 st;
#else
  struct stat st;
#endif
  int havepos;

  ((void)havepos);
  ((void)st);
  ((void)n);
  ((void)target);
  /* Make sure stdio is set up.  */

  CHECK_INIT (ptr, fp);

  _newlib_flockfile_start (fp);

  /* If we've been doing some writing, and we're in append mode
     then we don't really know where the filepos is.  */

  if (fp->_flags & __SAPP && fp->_flags & __SWR)
    {
      /* So flush the buffer and seek to the end.  */
      _fflush_r (ptr, fp);
    }

  /* Have to be able to seek.  */

  if ((seekfn = fp->_seek) == NULL)
    {
      ptr->_errno = ESPIPE;	/* ??? */
      _newlib_flockfile_exit (fp);
      return EOF;
    }

  /*
   * Change any SEEK_CUR to SEEK_SET, and check `whence' argument.
   * After this, whence is either SEEK_SET or SEEK_END.
   */

  switch (whence)
    {
    case SEEK_CUR:
      /*
       * In order to seek relative to the current stream offset,
       * we have to first find the current stream offset a la
       * ftell (see ftell for details).
       */
      _fflush_r (ptr, fp);   /* may adjust seek offset on append stream */
      if (fp->_flags & __SOFF)
	curoff = fp->_offset;
      else
	{
	  curoff = seekfn (ptr, fp->_cookie, (_fpos_t) 0, SEEK_CUR);
	  if (curoff == -1L)
	    {
	      _newlib_flockfile_exit (fp);
	      return EOF;
	    }
	}
      if (fp->_flags & __SRD)
	{
	  curoff -= fp->_r;
	  if (HASUB (fp))
	    curoff -= fp->_ur;
	}
      else if (fp->_flags & __SWR && fp->_p != NULL)
	curoff += fp->_p - fp->_bf._base;

      offset += curoff;
      whence = SEEK_SET;
      havepos = 1;
      break;

    case SEEK_SET:
    case SEEK_END:
      havepos = 0;
      break;

    default:
      ptr->_errno = EINVAL;
      _newlib_flockfile_exit (fp);
      return (EOF);
    }

  /*
   * Can only optimise if:
   *	reading (and not reading-and-writing);
   *	not unbuffered; and
   *	this is a `regular' Unix file (and hence seekfn==__sseek).
   * We must check __NBF first, because it is possible to have __NBF
   * and __SOPT both set.
   */

  if (fp->_bf._base == NULL)
    __smakebuf_r (ptr, fp);

#ifdef _FSEEK_OPTIMIZATION
  if (fp->_flags & (__SWR | __SRW | __SNBF | __SNPT))
    goto dumb;
  if ((fp->_flags & __SOPT) == 0)
    {
      if (seekfn != __sseek
	  || fp->_file < 0
#ifdef __USE_INTERNAL_STAT64
	  || _fstat64_r (ptr, fp->_file, &st)
#else
	  || _fstat_r (ptr, fp->_file, &st)
#endif
	  || (st.st_mode & S_IFMT) != S_IFREG)
	{
	  fp->_flags |= __SNPT;
	  goto dumb;
	}
#ifdef	HAVE_BLKSIZE
      fp->_blksize = st.st_blksize;
#else
      fp->_blksize = 1024;
#endif
      fp->_flags |= __SOPT;
    }

  /*
   * We are reading; we can try to optimise.
   * Figure out where we are going and where we are now.
   */

  if (whence == SEEK_SET)
    target = offset;
  else
    {
#ifdef __USE_INTERNAL_STAT64
      if (_fstat64_r (ptr, fp->_file, &st))
#else
      if (_fstat_r (ptr, fp->_file, &st))
#endif
	goto dumb;
      target = st.st_size + offset;
    }

  if (!havepos)
    {
      if (fp->_flags & __SOFF)
	curoff = fp->_offset;
      else
	{
	  curoff = seekfn (ptr, fp->_cookie, 0L, SEEK_CUR);
	  if (curoff == POS_ERR)
	    goto dumb;
	}
      curoff -= fp->_r;
      if (HASUB (fp))
	curoff -= fp->_ur;
    }

  /*
   * Compute the number of bytes in the input buffer (pretending
   * that any ungetc() input has been discarded).  Adjust current
   * offset backwards by this count so that it represents the
   * file offset for the first byte in the current input buffer.
   */

  if (HASUB (fp))
    {
      curoff += fp->_r;       /* kill off ungetc */
      n = fp->_up - fp->_bf._base;
      curoff -= n;
      n += fp->_ur;
    }
  else
    {
      n = fp->_p - fp->_bf._base;
      curoff -= n;
      n += fp->_r;
    }

  /*
   * If the target offset is within the current buffer,
   * simply adjust the pointers, clear EOF, undo ungetc(),
   * and return.
   */

  if (target >= curoff && target < curoff + n)
    {
      register int o = target - curoff;

      fp->_p = fp->_bf._base + o;
      fp->_r = n - o;
      if (HASUB (fp))
	FREEUB (ptr, fp);
      fp->_flags &= ~__SEOF;
      memset (&fp->_mbstate, 0, sizeof (_mbstate_t));
      _newlib_flockfile_exit (fp);
      return 0;
    }

  /*
   * The place we want to get to is not within the current buffer,
   * but we can still be kind to the kernel copyout mechanism.
   * By aligning the file offset to a block boundary, we can let
   * the kernel use the VM hardware to map pages instead of
   * copying bytes laboriously.  Using a block boundary also
   * ensures that we only read one block, rather than two.
   */

  curoff = target & ~(fp->_blksize - 1);
  if (seekfn (ptr, fp->_cookie, curoff, SEEK_SET) == POS_ERR)
    goto dumb;
  fp->_r = 0;
  fp->_p = fp->_bf._base;
  if (HASUB (fp))
    FREEUB (ptr, fp);
  fp->_flags &= ~__SEOF;
  n = target - curoff;
  if (n)
    {
      if (__srefill_r (ptr, fp) || fp->_r < n)
	goto dumb;
      fp->_p += n;
      fp->_r -= n;
    }
  memset (&fp->_mbstate, 0, sizeof (_mbstate_t));
  _newlib_flockfile_exit (fp);
  return 0;

  /*
   * We get here if we cannot optimise the seek ... just
   * do it.  Allow the seek function to change fp->_bf._base.
   */
#endif

#ifdef _FSEEK_OPTIMIZATION
dumb:
#endif
  if (_fflush_r (ptr, fp)
      || seekfn (ptr, fp->_cookie, offset, whence) == POS_ERR)
    {
      _newlib_flockfile_exit (fp);
      return EOF;
    }
  /* success: clear EOF indicator and discard ungetc() data */
  if (HASUB (fp))
    FREEUB (ptr, fp);
  fp->_p = fp->_bf._base;
  fp->_r = 0;
  /* fp->_w = 0; *//* unnecessary (I think...) */
  fp->_flags &= ~__SEOF;
  /* Reset no-optimization flag after successful seek.  The
     no-optimization flag may be set in the case of a read
     stream that is flushed which by POSIX/SUSv3 standards,
     means that a corresponding seek must not optimize.  The
     optimization is then allowed if no subsequent flush
     is performed.  */
  fp->_flags &= ~__SNPT;
  memset (&fp->_mbstate, 0, sizeof (_mbstate_t));
  _newlib_flockfile_end (fp);
  return 0;
}

#ifndef _REENT_ONLY

/**
 * @brief Repositions a file-position indicator in a stream.
 *
 * @details Sets the file-position indicator for the stream
 * pointed to by @fp.
 *
 * @return Returns 0 on sucess, otherwise, -1, and errno is
 * set to indicate the error.
 */
int fseeko(FILE *fp, _off_t offset, int whence)
{
  return _fseeko_r (_REENT, fp, offset, whence);
}

#endif /* !_REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
