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

/* Copyright 2005, 2007 Shaun Jackman
 * Permission to use, copy, modify, and distribute this software
 * is freely granted, provided that this notice is preserved.
 */
/* doc in dprintf.c */

#include <_ansi.h>
#include <reent.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <stdarg.h>
#include "local.h"

int _vdprintf_r(struct _reent *ptr, int fd, const char *restrict format,
  va_list ap)
{
  char *p;
  char buf[512];
  size_t n = sizeof buf;

  _REENT_SMALL_CHECK_INIT (ptr);
  p = _vasnprintf_r (ptr, buf, &n, format, ap);
  if (!p)
    return -1;
  n = _write_r (ptr, fd, p, n);
  if (p != buf)
    _free_r (ptr, p);
  return n;
}

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

#ifndef _REENT_ONLY

/**
 * @brief Formats output of a stdarg argument list.
 *
 * @details Same vfprintf behaviour except that instead of being
 * called with a variable number of arguments, is called with an
 * argument list as defined by <stdarg.h>.
 *
 * @return Same dprintf return.
 */
int vdprintf(int fd, const char *restrict format, va_list ap)
{
  return _vdprintf_r (_REENT, fd, format, ap);
}

#endif /* ! _REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
