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

#include <newlib.h>
#include <stdlib.h>
#include <wchar.h>
#include "local.h"

/**
 * @brief Gets number of bytes in a character.
 *
 * @details If @p s is not a null pointer, mblen() determines
 * the number of bytes constituting the character pointed to by @p s.
 *
 * @return If @p s is a null pointer, mblen() returns a non-zero or 0
 * value, if character encodings, respectively, do or do not have 
 * state-dependent encodings. If @p s is not a null pointer, mblen()
 * either returns 0 (if @p s points to the null byte), or returns the
 * number of bytes that constitute the character (if the next @p n or
 * fewer bytes form a valid character), or returns -1 (if they do not
 * form a valid character).
 */
int mblen(const char *s, size_t n)
{
#ifdef _MB_CAPABLE
  int retval = 0;
  struct _reent *reent = _REENT;
  mbstate_t *state;
  
  _REENT_CHECK_MISC(reent);
  state = &(_REENT_MBLEN_STATE(reent));
  retval = __mbtowc (reent, NULL, s, n, __locale_charset (), state);
  if (retval < 0)
    {
      state->__count = 0;
      return -1;
    }
  else
    return retval;
  
#else /* not _MB_CAPABLE */
  if (s == NULL || *s == '\0')
    return 0;
  if (n == 0)
    return -1;
  return 1;
#endif /* not _MB_CAPABLE */
}

#endif /* !_REENT_ONLY */
