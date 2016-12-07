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
#include <reent.h>
#include <stdio.h>
#include <string.h>
#include "local.h"

void _perror_r(struct _reent *ptr, const char *s)
{
  char *error;
  int dummy;

  _REENT_SMALL_CHECK_INIT (ptr);
  if (s != NULL && *s != '\0')
    {
      fputs (s, _stderr_r (ptr));
      fputs (": ", _stderr_r (ptr));
    }

  if ((error = _strerror_r (ptr, ptr->_errno, 1, &dummy)) != NULL)
    fputs (error, _stderr_r (ptr));

  fputc ('\n', _stderr_r (ptr));
}

#ifndef _REENT_ONLY

/**
 * @brief Writes error messages to standard error.
 *
 * @details Produces a message on the standard error output,
 * describing the last error encountered during a call to a
 * system or library function. First (if @p s is not NULL and
 * @p *s is not a null byte the argument string @p s is printed,
 * followed by a colon and a blank. Then the message and a new-line.
 */
void perror(const char *s)
{
  _perror_r (_REENT, s);
}

#endif
