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
