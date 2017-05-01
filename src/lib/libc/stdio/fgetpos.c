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

int _fgetpos_r(struct _reent * ptr, FILE *restrict fp, _fpos_t *restrict pos)
{
  *pos = _ftell_r (ptr, fp);

  if (*pos != -1)
    {
      return 0;
    }
  return 1;
}

#ifndef _REENT_ONLY

/**
 * @brief Gets current file position information.
 *
 * @details Stores the current values of the parse state
 * (if any) and file position indicator for the stream
 * pointed to by @p fp in the object pointed to by @p pos.
 * The value stored contains unspecified information usable
 * by fsetpos() for repositioning the stream to its position
 * at the time of the call to fgetpos().
 *
 * @return Returns 0; otherwise, returns a non-zero value and
 * sets errno to indicate the error.
 */
int fgetpos(FILE *restrict fp, _fpos_t *restrict pos)
{
  return _fgetpos_r (_REENT, fp, pos);
}

#endif /* !_REENT_ONLY */
