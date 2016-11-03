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

/* Nonzero if either X or Y is not aligned on a "long" boundary.  */
#define UNALIGNED(X, Y) \
  (((long)X & (sizeof (long) - 1)) | ((long)Y & (sizeof (long) - 1)))

/* DETECTNULL returns nonzero if (long)X contains a NULL byte. */
#if LONG_MAX == 2147483647L
#define DETECTNULL(X) (((X) - 0x01010101) & ~(X) & 0x80808080)
#else
#if LONG_MAX == 9223372036854775807L
#define DETECTNULL(X) (((X) - 0x0101010101010101) & ~(X) & 0x8080808080808080)
#else
#error long int is not a 32bit or 64bit type.
#endif
#endif

#ifndef DETECTNULL
#error long int is not a 32bit or 64bit byte
#endif

/**
 * @brief Compares part of two strings.
 *
 * @details Compares not more than n bytes (bytes that
 * follow a NUL character are not compared) from the 
 * array pointed to by @p s1 to the array pointed to by @p s2.
 *
 * @return Returns an integer greater than, equal to, or less 
 * than 0, if the possibly null-terminated array pointed to by
 * @p s1 is greater than, equal to, or less than the possibly 
 * null-terminated array pointed to by @p s2 respectively.
 */
int strncmp(const char *s1, const char *s2, size_t n)
{
#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
  if (n == 0)
    return 0;

  while (n-- != 0 && *s1 == *s2)
    {
      if (n == 0 || *s1 == '\0')
	break;
      s1++;
      s2++;
    }

  return (*(unsigned char *) s1) - (*(unsigned char *) s2);
#else
  unsigned long *a1;
  unsigned long *a2;

  if (n == 0)
    return 0;

  /* If s1 or s2 are unaligned, then compare bytes. */
  if (!UNALIGNED (s1, s2))
    {
      /* If s1 and s2 are word-aligned, compare them a word at a time. */
      a1 = (unsigned long*)s1;
      a2 = (unsigned long*)s2;
      while (n >= sizeof (long) && *a1 == *a2)
        {
          n -= sizeof (long);

          /* If we've run out of bytes or hit a null, return zero
	     since we already know *a1 == *a2.  */
          if (n == 0 || DETECTNULL (*a1))
	    return 0;

          a1++;
          a2++;
        }

      /* A difference was detected in last few bytes of s1, so search bytewise */
      s1 = (char*)a1;
      s2 = (char*)a2;
    }

  while (n-- > 0 && *s1 == *s2)
    {
      /* If we've run out of bytes or hit a null, return zero
	 since we already know *s1 == *s2.  */
      if (n == 0 || *s1 == '\0')
	return 0;
      s1++;
      s2++;
    }
  return (*(unsigned char *) s1) - (*(unsigned char *) s2);
#endif /* not PREFER_SIZE_OVER_SPEED */
}
