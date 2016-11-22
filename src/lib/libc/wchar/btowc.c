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

#include <wchar.h>
#include <stdlib.h>
#include <stdio.h>
#include <reent.h>
#include <string.h>
#include "../stdlib/local.h"

/**
 * @brief Single byte to wide character conversion.
 *
 * @details Determines whether @p c constitutes a valid (one-byte)
 * character in the initial shift state.
 *
 * @return Returns WEOF if @p c has the value EOF or if (unsigned char)
 * @p c does not constitute a valid (one-byte) character in the initial 
 * shift state. Otherwise, it shall return the wide-character representation
 * of that character.
 */
wint_t btowc(int c)
{
  mbstate_t mbs;
  int retval = 0;
  wchar_t pwc;
  unsigned char b;

  if (c == EOF)
    return WEOF;

  b = (unsigned char)c;

  /* Put mbs in initial state. */
  memset (&mbs, '\0', sizeof (mbs));

  _REENT_CHECK_MISC(_REENT);

  retval = __mbtowc (_REENT, &pwc, (const char *) &b, 1,
		     __locale_charset (), &mbs);

  if (retval != 0 && retval != 1)
    return WEOF;

  return (wint_t)pwc;
}
