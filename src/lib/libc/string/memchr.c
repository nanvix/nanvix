/*
 * Copyright(C) 2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

#include <_ansi.h>
#include <string.h>
#include <limits.h>

/* Nonzero if either X or Y is not aligned on a "long" boundary.  */
#define UNALIGNED(X) ((long)X & (sizeof (long) - 1))

/* How many bytes are loaded each iteration of the word copy loop.  */
#define LBLOCKSIZE (sizeof (long))

/* Threshhold for punting to the bytewise iterator.  */
#define TOO_SMALL(LEN)  ((LEN) < LBLOCKSIZE)

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

/* DETECTCHAR returns nonzero if (long)X contains the byte used
   to fill (long)MASK. */
#define DETECTCHAR(X,MASK) (DETECTNULL(X ^ MASK))

/**
 * @brief Finds byte in memory.
 *
 * @details Locates the first occurrence of @p c in the initial
 * @p length bytes pointed to by @p src_void.
 *
 * @return Returns a pointer to the located byte, or a null pointer
 * if the byte is not found.
 */
void *memchr(const void *src_void, int c, size_t length)
{
  const unsigned char *src = (const unsigned char *) src_void;
  unsigned char d = c;

#if !defined(PREFER_SIZE_OVER_SPEED) && !defined(__OPTIMIZE_SIZE__)
  unsigned long *asrc;
  unsigned long  mask;
  unsigned int i;

  while (UNALIGNED (src))
    {
      if (!length--)
        return NULL;
      if (*src == d)
        return (void *) src;
      src++;
    }

  if (!TOO_SMALL (length))
    {
      /* If we get this far, we know that length is large and src is
         word-aligned. */
      /* The fast code reads the source one word at a time and only
         performs the bytewise search on word-sized segments if they
         contain the search character, which is detected by XORing
         the word-sized segment with a word-sized block of the search
         character and then detecting for the presence of NUL in the
         result.  */
      asrc = (unsigned long *) src;
      mask = d << 8 | d;
      mask = mask << 16 | mask;
      for (i = 32; i < LBLOCKSIZE * 8; i <<= 1)
        mask = (mask << i) | mask;

      while (length >= LBLOCKSIZE)
        {
          if (DETECTCHAR (*asrc, mask))
            break;
          length -= LBLOCKSIZE;
          asrc++;
        }

      /* If there are fewer than LBLOCKSIZE characters left,
         then we resort to the bytewise loop.  */

      src = (unsigned char *) asrc;
    }

#endif /* not PREFER_SIZE_OVER_SPEED */

  while (length--)
    {
      if (*src == d)
        return (void *) src;
      src++;
    }

  return NULL;
}
