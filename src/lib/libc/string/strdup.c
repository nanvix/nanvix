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

#ifndef _REENT_ONLY

#include <reent.h>
#include <stdlib.h>
#include <string.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Duplicates a specific number of bytes from a string.
 *
 * @details Returns a pointer to a new string, which is a duplicate
 * of the string pointed to by @p str.
 *
 * @return Returns 0 in success, otherwise, an error number.
 */
char *strdup(const char *str)
{
  return _strdup_r (_REENT, str);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
#endif /* !_REENT_ONLY */
