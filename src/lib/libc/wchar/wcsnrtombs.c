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
#include <newlib.h>
#include <wchar.h>
#include <stdlib.h>
#include <stdio.h>
#include <errno.h>
#include "../stdlib/local.h"

size_t _wcsnrtombs_r(struct _reent *r, char *dst, const wchar_t **src, 
	size_t nwc, size_t len, mbstate_t *ps)
{
  char *ptr = dst;
  char buff[10];
  wchar_t *pwcs;
  size_t n;
  int i;

#ifdef _MB_CAPABLE
  if (ps == NULL)
    {
      _REENT_CHECK_MISC(r);
      ps = &(_REENT_WCSRTOMBS_STATE(r));
    }
#endif

  /* If no dst pointer, treat len as maximum possible value. */
  if (dst == NULL)
    len = (size_t)-1;

  n = 0;
  pwcs = (wchar_t *)(*src);

  while (n < len && nwc-- > 0)
    {
      int count = ps->__count;
      wint_t wch = ps->__value.__wch;
      int bytes = __wctomb (r, buff, *pwcs, __locale_charset (), ps);
      if (bytes == -1)
	{
	  r->_errno = EILSEQ;
	  ps->__count = 0;
	  return (size_t)-1;
	}
      if (n + bytes <= len)
	{
          n += bytes;
	  if (dst)
	    {
	      for (i = 0; i < bytes; ++i)
	        *ptr++ = buff[i];
	      ++(*src);
	    }
	  if (*pwcs++ == 0x00)
	    {
	      if (dst)
	        *src = NULL;
	      ps->__count = 0;
	      return n - 1;
	    }
	}
      else
	{
	  /* not enough room, we must back up state to before __wctomb call */
	  ps->__count = count;
	  ps->__value.__wch = wch;
          len = 0;
	}
    }

  return n;
} 

#ifndef _REENT_ONLY

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Converts a wide-character string to a character string (restartable);
 *
 * @details This function is equivalent to the wcsrtombs() function, except that
 * the conversion is limited to the first @p nwc wide characters.
 *
 * @return Returns the number of bytes in the resulting character sequence, not
 * including the terminating null (if any).
 */
size_t wcsnrtombs(char *restrict dst, const wchar_t **restrict src, size_t nwc, 
	size_t len, mbstate_t *restrict ps)
{
  return _wcsnrtombs_r (_REENT, dst, src, nwc, len, ps);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
#endif /* !_REENT_ONLY */
