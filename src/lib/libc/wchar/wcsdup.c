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
#include <wchar.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

wchar_t *_wcsdup_r(struct _reent *p, const wchar_t *str)
{
  size_t len = wcslen (str) + 1;
  wchar_t *copy = _malloc_r (p, len * sizeof (wchar_t));
  if (copy)
    wmemcpy (copy, str, len);
  return copy;
}

#ifndef _REENT_ONLY

/**
 * @brief Duplicates a wide-character string.
 *
 * @details Returns a pointer to a new wide-character string, 
 * allocated as if by a call to malloc(), which is the duplicate
 * of the wide-character string @p str. The returned pointer can 
 * be passed to free(). A null pointer is returned if the new 
 * wide-character string cannot be created.
 *
 * @return Returns a pointer to the newly allocated wide-character
 * string. Otherwise, it returns a null pointer and set errno to 
 * indicate the error.
 */
wchar_t *wcsdup(const wchar_t *str)
{
  return _wcsdup_r (_REENT, str);
}

#endif /* !_REENT_ONLY */
#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
