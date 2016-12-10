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
#include <stdio.h>
#include "local.h"

int _fgetc_r(struct _reent * ptr, FILE * fp)
{
  int result;
  CHECK_INIT(ptr, fp);
  _newlib_flockfile_start (fp);
  result = __sgetc_r (ptr, fp);
  _newlib_flockfile_end (fp);
  return result;
}

#ifndef _REENT_ONLY

/**
 * @brief Gets a byte from a stream.
 *
 * @details If the end-of-file indicator for the input
 * stream pointed to by @p fp is not set and a next byte
 * is present, the fgetc() function obtains the next byte
 * as an unsigned char converted to an int, from the input
 * stream pointed to by @p fp, and advance the associated file
 * position indicator for the stream (if defined). Since fgetc()
 * operates on bytes, reading a character consisting of multiple
 * bytes (or "a multi-byte character") may require multiple calls
 * to fgetc().
 *
 * @return Returns the next byte from the input stream pointed to
 * by @p fp. If the end-of-file indicator for the stream is set, or
 * if the stream is at end-of-file, the end-of-file indicator for
 * the stream is set and fgetc() returns EOF. If a read error occurs,
 * the error indicator for the stream is set, fgetc() returns EOF.
 */
int fgetc(FILE *fp)
{
#if !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__)
  int result;
  struct _reent *reent = _REENT;

  CHECK_INIT(reent, fp);
  _newlib_flockfile_start (fp);
  result = __sgetc_r (reent, fp);
  _newlib_flockfile_end (fp);
  return result;
#else
  return _fgetc_r (_REENT, fp);
#endif
}

#endif /* !_REENT_ONLY */
