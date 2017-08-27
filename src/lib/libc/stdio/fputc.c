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
#include <stdio.h>
#include "local.h"

int _fputc_r(struct _reent *ptr, int ch, FILE *file)
{
  int result;
  CHECK_INIT(ptr, file);
   _newlib_flockfile_start (file);
  result = _putc_r (ptr, ch, file);
  _newlib_flockfile_end (file);
  return result;
}

#ifndef _REENT_ONLY

/**
 * @brief Puts a byte on a stream.
 *
 * @details Writes the byte specified by @p ch (converted
 * to an unsigned char) to the output stream pointed to by
 * @p file, at the position indicated by the associated file-position
 * indicator for the stream (if defined), and advances the indicator
 * appropriately. If the file cannot support positioning requests,
 * or if the stream was opened with append mode, the byte is appended
 * to the output stream.
 *
 * @return Returns the value it has written. Otherwise, it returns EOF,
 * the error indicator for the stream shall be set.
 */
int fputc(int ch, FILE *file)
{
#if !defined(__OPTIMIZE_SIZE__) && !defined(PREFER_SIZE_OVER_SPEED)
  int result;
  struct _reent *reent = _REENT;

  CHECK_INIT(reent, file);
   _newlib_flockfile_start (file);
  result = _putc_r (reent, ch, file);
  _newlib_flockfile_end (file);
  return result;
#else
  return _fputc_r (_REENT, ch, file);
#endif
}

#endif /* !_REENT_ONLY */
