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

#if defined(LIBC_SCCS) && !defined(lint)
static char sccsid[] = "%W% (Berkeley) %G%";
#endif /* LIBC_SCCS and not lint */

#include <_ansi.h>
#include <stdio.h>
#include "local.h"

int _getc_r(struct _reent *ptr, register FILE *fp)
{
  int result;
  CHECK_INIT (ptr, fp);
  _newlib_flockfile_start (fp);
  result = __sgetc_r (ptr, fp);
  _newlib_flockfile_end (fp);
  return result;
}

#ifndef _REENT_ONLY

/**
 * @brief Gets a byte from a stream.
 *
 * @details The function is equivalent to fgetc().
 *
 * @return Returns the same value of fgetc().
 */
int getc(register FILE *fp)
{
  int result;
  struct _reent *reent = _REENT;

  CHECK_INIT (reent, fp);
  _newlib_flockfile_start (fp);
  result = __sgetc_r (reent, fp);
  _newlib_flockfile_end (fp);
  return result;
}

#endif /* !_REENT_ONLY */
