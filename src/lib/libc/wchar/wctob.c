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
#include <wchar.h>
#include <stdio.h>
#include <string.h>
#include <limits.h>
#include "../stdlib/local.h"

/**
 * @brief Wide-character to single-byte conversion.
 *
 * @details Determines whether @p wc corresponds to a member
 * of the extended character set whose character representation
 * is a single byte when in the initial shift state.
 *
 * @return Returns EOF if @p wc does not correspond to a character
 * with length one in the initial shift state. Otherwise, it returns
 * the single-byte representation of that character as an unsigned char
 * converted to int.
 */
int wctob(wint_t wc)
{
  struct _reent *reent;
  mbstate_t mbs;
  unsigned char pmb[MB_LEN_MAX];

  if (wc == WEOF)
    return EOF;

  /* Put mbs in initial state. */
  memset (&mbs, '\0', sizeof (mbs));

  reent = _REENT;
  _REENT_CHECK_MISC(reent);

  return __wctomb (reent, (char *) pmb, wc, __locale_charset (), &mbs) == 1
	  ? (int) pmb[0] : EOF;
}
