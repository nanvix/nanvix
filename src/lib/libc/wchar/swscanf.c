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
#include <wchar.h>
#include <stdarg.h>
#include "../stdio/local.h"

#ifndef _REENT_ONLY 

/**
 * @brief Converts formatted wide-character input.
 *
 * @details Reads from the wide-character string @p str. Each
 * function reads wide characters, interprets them according to
 * a format, and stores the results in its arguments. Each expects,
 * as arguments, a control wide-character string format described below,
 * and a set of pointer arguments indicating where the converted input
 * should be stored.
 *
 * @return Returns the number of successfully matched and assigned input
 * items; this number can be zero in the event of an early matching failure.
 */
int swscanf(const wchar_t *restrict str, const wchar_t *restrict fmt, ...)
{
  int ret;
  va_list ap;
  FILE f;

  f._flags = __SRD | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._r = wcslen (str) * sizeof (wchar_t);
  f._read = __seofread;
  f._ub._base = NULL;
  f._lb._base = NULL;
  f._file = -1;  /* No file. */
  va_start (ap, fmt);
  ret = __ssvfwscanf_r (_REENT, &f, fmt, ap);
  va_end (ap);
  return ret;
}

#endif /* !_REENT_ONLY */

int 
_swscanf_r (struct _reent *ptr, _CONST wchar_t *str, _CONST wchar_t *fmt, ...)
{
  int ret;
  va_list ap;
  FILE f;

  f._flags = __SRD | __SSTR;
  f._bf._base = f._p = (unsigned char *) str;
  f._bf._size = f._r = wcslen (str) * sizeof (wchar_t);
  f._read = __seofread;
  f._ub._base = NULL;
  f._lb._base = NULL;
  f._file = -1;  /* No file. */
  va_start (ap, fmt);
  ret = __ssvfwscanf_r (ptr, &f, fmt, ap);
  va_end (ap);
  return ret;
}
