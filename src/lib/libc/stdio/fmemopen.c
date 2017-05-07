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

/* Copyright (C) 2007 Eric Blake
 * Permission to use, copy, modify, and distribute this software
 * is freely granted, provided that this notice is preserved.
 */

#include <stdio.h>
#include <errno.h>
#include <string.h>
#include <sys/lock.h>
#include "local.h"

/* Describe details of an open memstream.  */
typedef struct fmemcookie {
  void *storage; /* storage to free on close */
  char *buf; /* buffer start */
  size_t pos; /* current position */
  size_t eof; /* current file size */
  size_t max; /* maximum file size */
  char append; /* nonzero if appending */
  char writeonly; /* 1 if write-only */
  char saved; /* saved character that lived at pos before write-only NUL */
} fmemcookie;

/* Read up to non-zero N bytes into BUF from stream described by
   COOKIE; return number of bytes read (0 on EOF).  */
static _READ_WRITE_RETURN_TYPE
_DEFUN(fmemreader, (ptr, cookie, buf, n),
       struct _reent *ptr _AND
       void *cookie _AND
       char *buf _AND
       _READ_WRITE_BUFSIZE_TYPE n)
{
  ((void)ptr);
  fmemcookie *c = (fmemcookie *) cookie;
  /* Can't read beyond current size, but EOF condition is not an error.  */
  if (c->pos > c->eof)
    return 0;
  if (n >= (int)(c->eof - c->pos))
    n = c->eof - c->pos;
  memcpy (buf, c->buf + c->pos, n);
  c->pos += n;
  return n;
}

/* Write up to non-zero N bytes of BUF into the stream described by COOKIE,
   returning the number of bytes written or EOF on failure.  */
static _READ_WRITE_RETURN_TYPE
_DEFUN(fmemwriter, (ptr, cookie, buf, n),
       struct _reent *ptr _AND
       void *cookie _AND
       const char *buf _AND
       _READ_WRITE_BUFSIZE_TYPE n)
{
  fmemcookie *c = (fmemcookie *) cookie;
  int adjust = 0; /* true if at EOF, but still need to write NUL.  */

  /* Append always seeks to eof; otherwise, if we have previously done
     a seek beyond eof, ensure all intermediate bytes are NUL.  */
  if (c->append)
    c->pos = c->eof;
  else if (c->pos > c->eof)
    memset (c->buf + c->eof, '\0', c->pos - c->eof);
  /* Do not write beyond EOF; saving room for NUL on write-only stream.  */
  if (c->pos + n > c->max - c->writeonly)
    {
      adjust = c->writeonly;
      n = c->max - c->pos;
    }
  /* Now n is the number of bytes being modified, and adjust is 1 if
     the last byte is NUL instead of from buf.  Write a NUL if
     write-only; or if read-write, eof changed, and there is still
     room.  When we are within the file contents, remember what we
     overwrite so we can restore it if we seek elsewhere later.  */
  if (c->pos + n > c->eof)
    {
      c->eof = c->pos + n;
      if (c->eof - adjust < c->max)
	c->saved = c->buf[c->eof - adjust] = '\0';
    }
  else if (c->writeonly)
    {
      if (n)
	{
	  c->saved = c->buf[c->pos + n - adjust];
	  c->buf[c->pos + n - adjust] = '\0';
	}
      else
	adjust = 0;
    }
  c->pos += n;
  if (n - adjust)
    memcpy (c->buf + c->pos - n, buf, n - adjust);
  else
    {
      ptr->_errno = ENOSPC;
      return EOF;
    }
  return n;
}

/* Seek to position POS relative to WHENCE within stream described by
   COOKIE; return resulting position or fail with EOF.  */
static _fpos_t
_DEFUN(fmemseeker, (ptr, cookie, pos, whence),
       struct _reent *ptr _AND
       void *cookie _AND
       _fpos_t pos _AND
       int whence)
{
  fmemcookie *c = (fmemcookie *) cookie;
#ifndef __LARGE64_FILES
  off_t offset = (off_t) pos;
#else /* __LARGE64_FILES */
  _off64_t offset = (_off64_t) pos;
#endif /* __LARGE64_FILES */

  if (whence == SEEK_CUR)
    offset += c->pos;
  else if (whence == SEEK_END)
    offset += c->eof;
  if (offset < 0)
    {
      ptr->_errno = EINVAL;
      offset = -1;
    }
  else if (offset > (off_t)c->max)
    {
      ptr->_errno = ENOSPC;
      offset = -1;
    }
#ifdef __LARGE64_FILES
  else if ((_fpos_t) offset != offset)
    {
      ptr->_errno = EOVERFLOW;
      offset = -1;
    }
#endif /* __LARGE64_FILES */
  else
    {
      if (c->writeonly && c->pos < c->eof)
	{
	  c->buf[c->pos] = c->saved;
	  c->saved = '\0';
	}
      c->pos = offset;
      if (c->writeonly && c->pos < c->eof)
	{
	  c->saved = c->buf[c->pos];
	  c->buf[c->pos] = '\0';
	}
    }
  return (_fpos_t) offset;
}

/* Seek to position POS relative to WHENCE within stream described by
   COOKIE; return resulting position or fail with EOF.  */
#ifdef __LARGE64_FILES
static _fpos64_t
_DEFUN(fmemseeker64, (ptr, cookie, pos, whence),
       struct _reent *ptr _AND
       void *cookie _AND
       _fpos64_t pos _AND
       int whence)
{
  _off64_t offset = (_off64_t) pos;
  fmemcookie *c = (fmemcookie *) cookie;
  if (whence == SEEK_CUR)
    offset += c->pos;
  else if (whence == SEEK_END)
    offset += c->eof;
  if (offset < 0)
    {
      ptr->_errno = EINVAL;
      offset = -1;
    }
  else if (offset > c->max)
    {
      ptr->_errno = ENOSPC;
      offset = -1;
    }
  else
    {
      if (c->writeonly && c->pos < c->eof)
	{
	  c->buf[c->pos] = c->saved;
	  c->saved = '\0';
	}
      c->pos = offset;
      if (c->writeonly && c->pos < c->eof)
	{
	  c->saved = c->buf[c->pos];
	  c->buf[c->pos] = '\0';
	}
    }
  return (_fpos64_t) offset;
}
#endif /* __LARGE64_FILES */

/* Reclaim resources used by stream described by COOKIE.  */
static int
_DEFUN(fmemcloser, (ptr, cookie),
       struct _reent *ptr _AND
       void *cookie)
{
  fmemcookie *c = (fmemcookie *) cookie;
  _free_r (ptr, c->storage);
  return 0;
}

/* Open a memstream around buffer BUF of SIZE bytes, using MODE.
   Return the new stream, or fail with NULL.  */
FILE *
_DEFUN(_fmemopen_r, (ptr, buf, size, mode),
       struct _reent *ptr _AND
       void *__restrict buf _AND
       size_t size _AND
       const char *__restrict mode)
{
  FILE *fp;
  fmemcookie *c;
  int flags;
  int dummy;

  if ((flags = __sflags (ptr, mode, &dummy)) == 0)
    return NULL;
  if (!size || !(buf || flags & __SRW))
    {
      ptr->_errno = EINVAL;
      return NULL;
    }
  if ((fp = __sfp (ptr)) == NULL)
    return NULL;
  if ((c = (fmemcookie *) _malloc_r (ptr, sizeof *c + (buf ? 0 : size)))
      == NULL)
    {
      _newlib_sfp_lock_start ();
      fp->_flags = 0;		/* release */
#ifndef __SINGLE_THREAD__
      __lock_close_recursive (fp->_lock);
#endif
      _newlib_sfp_lock_end ();
      return NULL;
    }

  c->storage = c;
  c->max = size;
  /* 9 modes to worry about.  */
  /* w/a, buf or no buf: Guarantee a NUL after any file writes.  */
  c->writeonly = (flags & __SWR) != 0;
  c->saved = '\0';
  if (!buf)
    {
      /* r+/w+/a+, and no buf: file starts empty.  */
      c->buf = (char *) (c + 1);
      c->buf[0] = '\0';
      c->pos = c->eof = 0;
      c->append = (flags & __SAPP) != 0;
    }
  else
    {
      c->buf = (char *) buf;
      switch (*mode)
	{
	case 'a':
	  /* a/a+ and buf: position and size at first NUL.  */
	  buf = memchr (c->buf, '\0', size);
	  c->eof = c->pos = buf ? (size_t)((char *) buf - c->buf) : size;
	  if (!buf && c->writeonly)
	    /* a: guarantee a NUL within size even if no writes.  */
	    c->buf[size - 1] = '\0';
	  c->append = 1;
	  break;
	case 'r':
	  /* r/r+ and buf: read at beginning, full size available.  */
	  c->pos = c->append = 0;
	  c->eof = size;
	  break;
	case 'w':
	  /* w/w+ and buf: write at beginning, truncate to empty.  */
	  c->pos = c->append = c->eof = 0;
	  *c->buf = '\0';
	  break;
	default:
	  abort ();
	}
    }

  _newlib_flockfile_start (fp);
  fp->_file = -1;
  fp->_flags = flags;
  fp->_cookie = c;
  fp->_read = flags & (__SRD | __SRW) ? fmemreader : NULL;
  fp->_write = flags & (__SWR | __SRW) ? fmemwriter : NULL;
  fp->_seek = fmemseeker;
#ifdef __LARGE64_FILES
  fp->_seek64 = fmemseeker64;
  fp->_flags |= __SL64;
#endif
  fp->_close = fmemcloser;
  _newlib_flockfile_end (fp);
  return fp;
}

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

#ifndef _REENT_ONLY

/**
 * @brief Opens a memory buffer stream.
 *
 * @details Associates the buffer given by the
 * @p buf and @p size arguments with a stream.
 *
 * @return Returns a pointer to the object controlling
 * the stream. Otherwise, a null pointer is returned,
 * and errno is set to indicate the error.
 */
FILE *fmemopen(void *restrict buf, size_t size, const char *restrict mode)
{
  return _fmemopen_r (_REENT, buf, size, mode);
}

#endif /* !_REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
