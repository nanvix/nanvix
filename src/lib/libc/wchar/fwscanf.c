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
 * @details Reads from the named input @p fp.
 *
 * @return Returns the number of successfully matched and 
 * assigned input items; this number can be zero in the event
 * of an early matching failure. If the input ends before the
 * first conversion (if any) has completed, and without a matching
 * failure having occurred, EOF is returned. If an error occurs
 * before the first conversion (if any) has completed, and without a
 * matching failure having occurred, EOF is returned and errno is set
 * to indicate the error. If a read error occurs, the error indicator
 * for the stream is set.
 */
int fwscanf(FILE *restrict fp, const wchar_t *restrict fmt, ...)
{
  int ret;
  va_list ap;

  va_start (ap, fmt);
  ret = _vfwscanf_r (_REENT, fp, fmt, ap);
  va_end (ap);
  return ret;
}

#endif /* !_REENT_ONLY */

int _fwscanf_r(struct _reent *ptr, FILE *fp, const wchar_t *fmt, ...)
{
  int ret;
  va_list ap;

  va_start (ap, fmt);
  ret = _vfwscanf_r (ptr, fp, fmt, ap);
  va_end (ap);
  return (ret);
}
