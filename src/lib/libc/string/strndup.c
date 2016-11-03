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

#include <_ansi.h>
#include <reent.h>
#include <stdlib.h>
#include <string.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Duplicates a specific number of bytes from a string
 *
 * @details Duplicates the provided @p str in a new block 
 * of memory allocated but copies at most @p size plus one bytes
 * into the newly allocated memory, terminating the new string 
 * with a NUL character.
 *
 * @return Returns a pointer to the newly allocated memory containing
 * the duplicate string. Otherwise, it returns a null pointer.
 */
char *strndup(const char *str, size_t n)
{
  return _strndup_r (_REENT, str, n);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
#endif /* !_REENT_ONLY */
