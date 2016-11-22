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

/*-
 * Code created by modifying scanf.c which has following copyright.
 *
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
#ifdef _HAVE_STDC
#include <stdarg.h>
#else
#include <varargs.h>
#endif
#include "../stdio/local.h"

#ifndef _REENT_ONLY

/**
 * @brief Wide-character formatted input of a stdarg argument list.
 *
 * @details This function is equivalent to wscanf() except that 
 * instead of being called with a variable number of arguments, they
 * are called with an argument list as defined by <stdarg.h>.
 *
 * @return Returns the number of successfully matched and assigned input items.
 */
int vwscanf(const wchar_t *restrict fmt, va_list ap)
{
  struct _reent *reent = _REENT;

  _REENT_SMALL_CHECK_INIT (reent);
  return __svfwscanf_r (reent, _stdin_r (reent), fmt, ap);
}

#endif /* !_REENT_ONLY */

int _vwscanf_r(struct _reent *ptr, const wchar_t *fmt, va_list ap)
{
  _REENT_SMALL_CHECK_INIT (ptr);
  return __svfwscanf_r (ptr, _stdin_r (ptr), fmt, ap);
}

