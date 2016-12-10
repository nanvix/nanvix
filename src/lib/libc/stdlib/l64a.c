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

/* l64a - convert long to radix-64 ascii string
 *          
 * Conversion is performed on at most 32-bits of input value starting 
 * from least significant bits to the most significant bits.
 *
 * The routine splits the input value into groups of 6 bits for up to 
 * 32 bits of input.  This means that the last group may be 2 bits 
 * (bits 30 and 31).
 * 
 * Each group of 6 bits forms a value from 0-63 which is converted into 
 * a character as follows:
 *         0 = '.'
 *         1 = '/'
 *         2-11 = '0' to '9'
 *        12-37 = 'A' to 'Z'
 *        38-63 = 'a' to 'z'
 *
 * When the remaining bits are zero or all 32 bits have been translated, 
 * a nul terminator is appended to the resulting string.  An input value of 
 * 0 results in an empty string.
 */

#include <_ansi.h>
#include <stdlib.h>
#include <reent.h>

#ifdef _XOPEN_SOURCE

static const char R64_ARRAY[] = "./0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

char *_l64a_r(struct _reent *rptr, long value)
{
  char *ptr;
  char *result;
  int i, index;
  unsigned long tmp = (unsigned long)value & 0xffffffff;

  _REENT_CHECK_MISC(rptr);
  result = _REENT_L64A_BUF(rptr);
  ptr = result;

  for (i = 0; i < 6; ++i)
    {
      if (tmp == 0)
  {
    *ptr = '\0';
    break;
  }

      index = tmp & (64 - 1);
      *ptr++ = R64_ARRAY[index];
      tmp >>= 6;
    }

  return result;
}

/**
 * @brief Converts between a 32-bit integer and a radix-64 ASCII string.
 *
 * @details The function takes a long argument and return a pointer to
 * the corresponding radix-64 representation. The behavior of l64a() is
 * unspecified if @p value is negative.
 *
 * @return Returns the long value resulting from conversion of the input 
 * string.
 */
char *l64a(long value)
{
  return _l64a_r (_REENT, value);
}

#endif /* _XOPEN_SOURCE */
