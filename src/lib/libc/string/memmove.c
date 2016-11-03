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
#include <_ansi.h>
#include <stddef.h>
#include <limits.h>
#include "local.h"

/* Nonzero if either X or Y is not aligned on a "long" boundary.  */
#define UNALIGNED(X, Y) \
  (((long)X & (sizeof (long) - 1)) | ((long)Y & (sizeof (long) - 1)))

/* How many bytes are copied each iteration of the 4X unrolled loop.  */
#define BIGBLOCKSIZE    (sizeof (long) << 2)

/* How many bytes are copied each iteration of the word copy loop.  */
#define LITTLEBLOCKSIZE (sizeof (long))

/* Threshhold for punting to the byte copier.  */
#define TOO_SMALL(LEN)  ((LEN) < BIGBLOCKSIZE)

/**
 * @brief Copies bytes in memory with overlapping areas.
 *
 * @details Copies @p length bytes from the object pointed to
 * by @p src_void into the object pointed to by @p dst_void.
 *
 * @return Returns @p dst_void.
 */
void *memmove(void *dst_void, const void *src_void, size_t length)
{
#if defined(PREFER_SIZE_OVER_SPEED) || defined(__OPTIMIZE_SIZE__)
  char *dst = dst_void;
  _CONST char *src = src_void;

  if (src < dst && dst < src + length)
    {
      /* Have to copy backwards */
      src += length;
      dst += length;
      while (length--)
	{
	  *--dst = *--src;
	}
    }
  else
    {
      while (length--)
	{
	  *dst++ = *src++;
	}
    }

  return dst_void;
#else
  char *dst = dst_void;
  _CONST char *src = src_void;
  long *aligned_dst;
  _CONST long *aligned_src;

  if (src < dst && dst < src + length)
    {
      /* Destructive overlap...have to copy backwards */
      src += length;
      dst += length;
      while (length--)
	{
	  *--dst = *--src;
	}
    }
  else
    {
      /* Use optimizing algorithm for a non-destructive copy to closely 
         match memcpy. If the size is small or either SRC or DST is unaligned,
         then punt into the byte copy loop.  This should be rare.  */
      if (!TOO_SMALL(length) && !UNALIGNED (src, dst))
        {
          aligned_dst = (long*)dst;
          aligned_src = (long*)src;

          /* Copy 4X long words at a time if possible.  */
          while (length >= BIGBLOCKSIZE)
            {
              *aligned_dst++ = *aligned_src++;
              *aligned_dst++ = *aligned_src++;
              *aligned_dst++ = *aligned_src++;
              *aligned_dst++ = *aligned_src++;
              length -= BIGBLOCKSIZE;
            }

          /* Copy one long word at a time if possible.  */
          while (length >= LITTLEBLOCKSIZE)
            {
              *aligned_dst++ = *aligned_src++;
              length -= LITTLEBLOCKSIZE;
            }

          /* Pick up any residual with a byte copier.  */
          dst = (char*)aligned_dst;
          src = (char*)aligned_src;
        }

      while (length--)
        {
          *dst++ = *src++;
        }
    }

  return dst_void;
#endif /* not PREFER_SIZE_OVER_SPEED */
}
