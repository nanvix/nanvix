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

#include <string.h>
#include <limits.h>

/*SUPPRESS 560*/
/*SUPPRESS 530*/

/* Nonzero if either X or Y is not aligned on a "long" boundary.  */
#define UNALIGNED(X, Y) \
  (((long)X & (sizeof (long) - 1)) | ((long)Y & (sizeof (long) - 1)))

#if LONG_MAX == 2147483647L
#define DETECTNULL(X) (((X) - 0x01010101) & ~(X) & 0x80808080)
#else
#if LONG_MAX == 9223372036854775807L
/* Nonzero if X (a long int) contains a NULL byte. */
#define DETECTNULL(X) (((X) - 0x0101010101010101) & ~(X) & 0x8080808080808080)
#else
#error long int is not a 32bit or 64bit type.
#endif
#endif

#ifndef DETECTNULL
#error long int is not a 32bit or 64bit byte
#endif

#define TOO_SMALL(LEN) ((LEN) < sizeof (long))

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Copies fixed length string, returning a pointer to the array end.
 *
 * @details Copies not more than @p count bytes (bytes that follow a NUL
 * character are not copied) from the array pointed to by @p src to the
 * array pointed to by @p dst.
 *
 * @return If a NUL character is written to the destination, returns the 
 * address of the first such NUL character. Otherwise, returns &dst[n].
 */
char *stpncpy(char *restrict dst, const char *restrict src, size_t count)
{
  char *ret = NULL;

#if !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__)
  long *aligned_dst;
  _CONST long *aligned_src;

  /* If SRC and DEST is aligned and count large enough, then copy words.  */
  if (!UNALIGNED (src, dst) && !TOO_SMALL (count))
    {
      aligned_dst = (long*)dst;
      aligned_src = (long*)src;

      /* SRC and DEST are both "long int" aligned, try to do "long int"
	 sized copies.  */
      while (count >= sizeof (long int) && !DETECTNULL(*aligned_src))
	{
	  count -= sizeof (long int);
	  *aligned_dst++ = *aligned_src++;
	}

      dst = (char*)aligned_dst;
      src = (char*)aligned_src;
    }
#endif /* not PREFER_SIZE_OVER_SPEED */

  while (count > 0)
    {
      --count;
      if ((*dst++ = *src++) == '\0')
	{
	  ret = dst - 1;
	  break;
	}
    }

  while (count-- > 0)
    *dst++ = '\0';

  return ret ? ret : dst;
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
