/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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
 * Copyright (c) 1990, 2007 The Regents of the University of California.
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
#include <wchar.h>
#include <stdarg.h>
#include <limits.h>
#include <errno.h>
#include "../stdio/local.h"

/* NOTE:  _swprintf_r() should be identical to swprintf() except for the
 * former having ptr as a parameter and the latter needing to declare it as
 * a variable set to _REENT.  */

int _swprintf_r(struct _reent *ptr, wchar_t *str, size_t size,
	const wchar_t *fmt, ...)
{
  int ret;
  va_list ap;
  FILE f;

  if (size > INT_MAX / sizeof (wchar_t))
    {
      ptr->_errno = EOVERFLOW;	/* POSIX extension */
      return EOF;
    }
  f._flags = __SWR | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._w = (size > 0 ? (size - 1) * sizeof (wchar_t) : 0);
  f._file = -1;  /* No file. */
  va_start (ap, fmt);
  ret = _svfwprintf_r (ptr, &f, fmt, ap);
  va_end (ap);
  /* _svfwprintf_r() does not put in a terminating NUL, so add one if
   * appropriate, which is whenever size is > 0.  _svfwprintf_r() stops
   * after n-1, so always just put at the end.  */
  if (size > 0)  {
    *(wchar_t *)f._p = L'\0';	/* terminate the string */
  }
  if((size_t)ret >= size)  {
    /* _svfwprintf_r() returns how many wide characters it would have printed
     * if there were enough space.  Return an error if too big to fit in str,
     * unlike snprintf, which returns the size needed.  */
    ptr->_errno = EOVERFLOW;	/* POSIX extension */
    ret = -1;
  }
  return (ret);
}

#ifndef _REENT_ONLY

/**
 * @brief Prints formatted wide-character output.
 *
 * @details Places output followed by the null wide character
 * in consecutive wide characters starting at @p *ws; no more
 * than @p size wide characters is written, including a terminating
 * null wide character, which is always added (unless @p size is zero).
 *
 * @return Returns the number of wide characters transmitted, excluding
 * the terminating null wide character, or a negative value if an output
 * error was encountered, and set errno to indicate the error.
 */
int swprintf(wchar_t *restrict str, size_t size, const wchar_t *restrict fmt,
	...)
{
  int ret;
  va_list ap;
  FILE f;
  struct _reent *ptr = _REENT;

  if (size > INT_MAX / sizeof (wchar_t))
    {
      ptr->_errno = EOVERFLOW;	/* POSIX extension */
      return EOF;
    }
  f._flags = __SWR | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._w = (size > 0 ? (size - 1) * sizeof (wchar_t) : 0);
  f._file = -1;  /* No file. */
  va_start (ap, fmt);
  ret = _svfwprintf_r (ptr, &f, fmt, ap);
  va_end (ap);
  /* _svfwprintf_r() does not put in a terminating NUL, so add one if
   * appropriate, which is whenever size is > 0.  _svfwprintf_r() stops
   * after n-1, so always just put at the end.  */
  if (size > 0)  {
    *(wchar_t *)f._p = L'\0';	/* terminate the string */
  }
  if((size_t)ret >= size)  {
    /* _svfwprintf_r() returns how many wide characters it would have printed
     * if there were enough space.  Return an error if too big to fit in str,
     * unlike snprintf, which returns the size needed.  */
    ptr->_errno = EOVERFLOW;	/* POSIX extension */
    ret = -1;
  }
  return (ret);
}

#endif
