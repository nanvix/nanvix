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
/* doc in sprintf.c */
/* This code created by modifying sprintf.c so copyright inherited. */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <stdarg.h>
#include <limits.h>
#include <errno.h>
#include "local.h"

int _snprintf_r(struct _reent *ptr, char *restrict str, size_t size,
  const char *restrict fmt, ...)
{
  int ret;
  va_list ap;
  FILE f;

  if (size > INT_MAX)
    {
      ptr->_errno = EOVERFLOW;
      return EOF;
    }
  f._flags = __SWR | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._w = (size > 0 ? size - 1 : 0);
  f._file = -1;  /* No file. */

  va_start (ap, fmt);
  ret = _svfprintf_r (ptr, &f, fmt, ap);
  va_end (ap);
  if (ret < EOF)
    ptr->_errno = EOVERFLOW;
  if (size > 0)
    *f._p = 0;
  return (ret);
}

#ifndef _REENT_ONLY

/**
 * @brief Prints formatted output.
 *
 * @details The snprintf() function is equivalent to sprintf(),
 * with the addition of the @p size argument which states the size
 * of the buffer referred to by @p str. If @p size is zero, nothing
 * is written and @p str may be a null pointer. Otherwise, output bytes
 * beyond the n-1st is discarded instead of being written to the array,
 * and a null byte is written at the end of the bytes actually written into
 * the array.
 *
 * @return Returns number of bytes that would be written to @p str had @p size
 * been sufficiently large excluding the terminating null byte.
 */
int snprintf(char *restrict str, size_t size, const char *restrict fmt, ...)
{
  int ret;
  va_list ap;
  FILE f;
  struct _reent *ptr = _REENT;

  if (size > INT_MAX)
    {
      ptr->_errno = EOVERFLOW;
      return EOF;
    }
  f._flags = __SWR | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._w = (size > 0 ? size - 1 : 0);
  f._file = -1;  /* No file. */

  va_start (ap, fmt);
  ret = _svfprintf_r (ptr, &f, fmt, ap);
  va_end (ap);
  if (ret < EOF)
    ptr->_errno = EOVERFLOW;
  if (size > 0)
    *f._p = 0;
  return (ret);
}

#endif
