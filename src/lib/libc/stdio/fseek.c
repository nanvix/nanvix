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
#include <errno.h>
#include "local.h"

int _fseek_r(struct _reent *ptr, register FILE *fp, long offset, int whence)
{
  return _fseeko_r (ptr, fp, offset, whence);
}

#ifndef _REENT_ONLY

/**
 * @brief Repositions a file-position indicator in a stream.
 *
 * @details Sets the file position indicator for the stream
 * pointed to by @p fp. The new position, measured in bytes,
 * is obtained by adding offset bytes to the position specified
 * by whence. If whence is set to SEEK_SET, SEEK_CUR, or SEEK_END,
 * the offset is relative to the start of the file, the current
 * position indicator, or end-of-file, respectively. A successful
 * call to the fseek() function clears the end-of-file indicator
 * for the stream and undoes any effects of the ungetc() function
 * on the same stream.
 *
 * @return Returns 0 if they succeed. Otherwise, returns -1 and sets
 * errno to indicate the error.
 */
int fseek(register FILE *fp, long offset, int whence)
{
  return _fseek_r (_REENT, fp, offset, whence);
}

#endif /* !_REENT_ONLY */
