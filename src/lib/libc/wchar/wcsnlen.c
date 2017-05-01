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
 * Copyright (c) 2003, Artem B. Bityuckiy (dedekind@mail.ru).
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the above copyright notice,
 * this condition statement, and the following disclaimer are retained
 * in any redistributions of the source code.
 *
 * THIS SOFTWARE IS PROVIDED BY THE AUTHOR AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE AUTHOR OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

#include <_ansi.h>
#include <sys/types.h>
#include <wchar.h>

#if defined(_POSIX_C_SOURCE) || defined(_XOPEN_SOURCE)

/**
 * @brief Gets length of a fixed-sized wide-character string.
 *
 * @details Computes the smaller of the number of wide characters in
 * the array to which @p s points, not including any terminating null
 * wide-character code, and the value of @p maxlen.
 *
 * @return Returns the number of wide characters preceding the first
 * null wide-character code in the array to which @p s points, if @p s
 * contains a null wide-character code within the first maxlen wide 
 * characters; otherwise, it returns maxlen.
 */
size_t wcsnlen(const wchar_t *s, size_t maxlen)
{
  const wchar_t *p;

  p = s;
  while (*p && maxlen-- > 0)
    p++;

  return (size_t)(p - s);
}

#endif /* _POSIX_C_SOURCE || _XOPEN_SOURCE */
