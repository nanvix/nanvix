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

#include <reent.h>
#include <stdlib.h>
#include <string.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

char *_strdup_r(struct _reent *reent_ptr, const char *str)
{
  size_t len = strlen (str) + 1;
  char *copy = _malloc_r (reent_ptr, len);
  if (copy)
    {
      memcpy (copy, str, len);
    }
  return copy;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
