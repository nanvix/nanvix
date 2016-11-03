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

#include <_ansi.h>
#include <stddef.h>
#include <string.h>
#include <limits.h>

/* Nonzero if either X or Y is not aligned on a "long" boundary.  */
#define UNALIGNED(X, Y) \
  (((long)X & (sizeof (long) - 1)) | ((long)Y & (sizeof (long) - 1)))

/* How many bytes are copied each iteration of the word copy loop.  */
#define LITTLEBLOCKSIZE (sizeof (long))

/* Threshhold for punting to the byte copier.  */
#define TOO_SMALL(LEN)  ((LEN) < LITTLEBLOCKSIZE)

/* Macros for detecting endchar */
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

#ifdef _XOPEN_SOURCE

/**
 * @brief Copies bytes in memory.
 *
 * @details Copies bytes from memory area @p src0 into @p dst0,
 * stopping after the first occurrence of byte @p endchar0 
 * (converted to an unsigned char) is copied, or after @p len0
 * bytes are copied, whichever comes first.
 *
 * @return Returns a pointer to the byte after the copy of @p endchar 
 * in @p dst0, or a null pointer if @p endchar0 was not found in the
 * first @p len0 bytes of @p src0.
 */
void *memccpy(void *restrict dst0, const void* restrict src0, int endchar0, 
  size_t len0)
{

#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
  _PTR ptr = NULL;
  char *dst = (char *) dst0;
  char *src = (char *) src0;
  char endchar = endchar0 & 0xff;

  while (len0--)
    {
      if ((*dst++ = *src++) == endchar)
        {
          ptr = dst;
          break;
        }
    }

  return ptr;
#else
  _PTR ptr = NULL;
  char *dst = dst0;
  _CONST char *src = src0;
  long *aligned_dst;
  _CONST long *aligned_src;
  char endchar = endchar0 & 0xff;

  /* If the size is small, or either SRC or DST is unaligned,
     then punt into the byte copy loop.  This should be rare.  */
  if (!TOO_SMALL(len0) && !UNALIGNED (src, dst))
    {
      unsigned int i;
      unsigned long mask = 0;

      aligned_dst = (long*)dst;
      aligned_src = (long*)src;

      /* The fast code reads the ASCII one word at a time and only
         performs the bytewise search on word-sized segments if they
         contain the search character, which is detected by XORing
         the word-sized segment with a word-sized block of the search
         character and then detecting for the presence of NULL in the
         result.  */
      for (i = 0; i < LITTLEBLOCKSIZE; i++)
        mask = (mask << 8) + endchar;


      /* Copy one long word at a time if possible.  */
      while (len0 >= LITTLEBLOCKSIZE)
        {
          unsigned long buffer = (unsigned long)(*aligned_src);
          buffer ^=  mask;
          if (DETECTNULL (buffer))
            break; /* endchar is found, go byte by byte from here */
          *aligned_dst++ = *aligned_src++;
          len0 -= LITTLEBLOCKSIZE;
        }

       /* Pick up any residual with a byte copier.  */
      dst = (char*)aligned_dst;
      src = (char*)aligned_src;
    }

  while (len0--)
    {
      if ((*dst++ = *src++) == endchar)
        {
          ptr = dst;
          break;
        }
    }

  return ptr;
#endif /* not PREFER_SIZE_OVER_SPEED */
}

#endif /* _XOPEN_SOURCE */
