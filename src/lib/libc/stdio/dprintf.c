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

/* Copyright 2005, 2007 Shaun Jackman
 * Permission to use, copy, modify, and distribute this software
 * is freely granted, provided that this notice is preserved.
 */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <unistd.h>
#include <stdarg.h>
#include "local.h"

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

int _dprintf_r(struct _reent *ptr, int fd, const char *restrict format, ...)
{
	va_list ap;
	int n;
	_REENT_SMALL_CHECK_INIT (ptr);
	va_start (ap, format);
	n = _vdprintf_r (ptr, fd, format, ap);
	va_end (ap);
	return n;
}

#ifndef _REENT_ONLY

/**
 * @brief Prints formatted output.
 *
 * @details The function dprintf is exact analog except that
 * output to a file descriptor fd instead of to a stdio stream.
 *
 * @return Returns the number of bytes transmitted.
 */
int dprintf(int fd, const char *restrict format, ...)
{
  va_list ap;
  int n;
  struct _reent *ptr = _REENT;

  _REENT_SMALL_CHECK_INIT (ptr);
  va_start (ap, format);
  n = _vdprintf_r (ptr, fd, format, ap);
  va_end (ap);
  return n;
}

#endif /* ! _REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
