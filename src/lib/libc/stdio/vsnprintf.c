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
/* doc in vfprintf.c */

#if defined(LIBC_SCCS) && !defined(lint)
static char sccsid[] = "%W% (Berkeley) %G%";
#endif /* LIBC_SCCS and not lint */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <limits.h>
#include <stdarg.h>
#include <errno.h>

#include "local.h"

#ifndef _REENT_ONLY

int _vsnprintf_r(struct _reent *ptr, char *restrict str, size_t size,
  const char *restrict fmt, va_list ap)
{
  int ret;
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
  ret = _svfprintf_r (ptr, &f, fmt, ap);
  if (ret < EOF)
    ptr->_errno = EOVERFLOW;
  if (size > 0)
    *f._p = 0;
  return ret;
}

/**
 * @brief Formats output of a stdarg argument list.
 *
 * @details Same snprintf behaviour except that instead 
 * of being called with a variable number of arguments,
 * is called with an argument list as defined by <stdarg.h>.
 *
 * @return Same snprintf return.
 */
int vsnprintf(char *restrict str, size_t size, const char *restrict fmt,
  va_list ap)
{
  return _vsnprintf_r (_REENT, str, size, fmt, ap);
}

#endif /* !_REENT_ONLY */
