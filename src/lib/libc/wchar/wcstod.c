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

/*-
 * Copyright (c) 2002 Tim J. Robbins
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
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
#include <errno.h>
#include <stdlib.h>
#include <string.h>
#include <wchar.h>
#include <wctype.h>
#include <locale.h>
#include <math.h>

double _wcstod_r(struct _reent *ptr, const wchar_t *nptr, wchar_t **endptr)
{
        static const mbstate_t initial;
        mbstate_t mbs;
        double val;
        char *buf, *end;
        const wchar_t *wcp;
        size_t len;

        while (iswspace(*nptr))
                nptr++;

        /*
         * Convert the supplied numeric wide char. string to multibyte.
         *
         * We could attempt to find the end of the numeric portion of the
         * wide char. string to avoid converting unneeded characters but
         * choose not to bother; optimising the uncommon case where
         * the input string contains a lot of text after the number
         * duplicates a lot of strtod()'s functionality and slows down the
         * most common cases.
         */
        wcp = nptr;
        mbs = initial;
        if ((len = _wcsrtombs_r(ptr, NULL, &wcp, 0, &mbs)) == (size_t)-1) {
                if (endptr != NULL)
                        *endptr = (wchar_t *)nptr;
                return (0.0);
        }
        if ((buf = _malloc_r(ptr, len + 1)) == NULL)
                return (0.0);
        mbs = initial;
        _wcsrtombs_r(ptr, buf, &wcp, len + 1, &mbs);

        /* Let strtod() do most of the work for us. */
        val = _strtod_r(ptr, buf, &end);

        /*
         * We only know where the number ended in the _multibyte_
         * representation of the string. If the caller wants to know
         * where it ended, count multibyte characters to find the
         * corresponding position in the wide char string.
         */
        if (endptr != NULL) {
		/* The only valid multibyte char in a float converted by
		   strtod/wcstod is the radix char.  What we do here is,
		   figure out if the radix char was in the valid leading
		   float sequence in the incoming string.  If so, the
		   multibyte float string is strlen(radix char) - 1 bytes
		   longer than the incoming wide char string has characters.
		   To fix endptr, reposition end as if the radix char was
		   just one byte long.  The resulting difference (end - buf)
		   is then equivalent to the number of valid wide characters
		   in the input string. */
		len = strlen (_localeconv_r (ptr)->decimal_point);
		if (len > 1) {
			char *d = strstr (buf,
					  _localeconv_r (ptr)->decimal_point);
			if (d && d < end)
				end -= len - 1;
		}
                *endptr = (wchar_t *)nptr + (end - buf);
	}

        _free_r(ptr, buf);

        return (val);
}

float _wcstof_r(struct _reent *ptr, const wchar_t *nptr, wchar_t **endptr)
{
  double retval = _wcstod_r (ptr, nptr, endptr);
  if (isnan (retval))
    return nanf (NULL);
  return (float)retval;
}

#ifndef _REENT_ONLY

/**
 * @brief Converts a wide-character string to a double-precision number.
 *
 * @details Converts the initial portion of the wide-character string 
 * pointed to by @p nptr to double representation.
 *
 * @return Returns the converted value. If no conversion could be performed,
 * 0 shall be returned.
 */
double wcstod(const wchar_t *restrict nptr, wchar_t **restrict endptr)
{
  return _wcstod_r (_REENT, nptr, endptr);
}

/**
 * @brief Converts a wide-character string to a double-precision number.
 *
 * @details Converts the initial portion of the wide-character string 
 * pointed to by @p nptr to float representation.
 *
 * @return Returns the converted value. If no conversion could be performed,
 * 0 shall be returned.
 */
float wcstof(const wchar_t *restrict nptr, wchar_t **restrict endptr)
{
  double retval = _wcstod_r (_REENT, nptr, endptr);
  if (isnan (retval))
    return nanf (NULL);
  return (float)retval;
}

#endif
