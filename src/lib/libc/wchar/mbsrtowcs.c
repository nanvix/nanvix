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

size_t _mbsrtowcs_r(struct _reent *r, wchar_t *dst, const char **src,
	size_t len, mbstate_t *ps)
{
  return _mbsnrtowcs_r (r, dst, src, (size_t) -1, len, ps);
}

#ifndef _REENT_ONLY

/**
 * @brief Converts a character string to a wide-character string (restartable).
 *
 * @details Converts a sequence of characters, beginning in the conversion 
 * state described by the object pointed to by @p ps, from the array indirectly
 * pointed to by @p src into a sequence of corresponding wide characters. If
 * @p dst is not a null pointer, the converted characters is stored into the
 * array pointed to by @p dst. Conversion continues up to and including a 
 * terminating null character, which is also be stored. Conversion stops early
 * in either of the following cases: A sequence of bytes is encountered that does
 * not form a valid character. @p len codes have been stored into the array pointed
 * to by @p dst (and @p dst is not a null pointer).
 *
 * @return If the input conversion encounters a sequence of bytes that do not form 
 * a valid character, an encoding error occurs. In this case, these functions stores
 * the value of the macro [EILSEQ] in errno and returns (size_t)-1; the conversion
 * state is undefined. Otherwise, the function returns the number of characters
 * successfully converted, not including the terminating null (if any).
 */
size_t mbsrtowcs(wchar_t *restrict dst, const char **restrict src, size_t len, 
	mbstate_t *restrict ps)
{
  return _mbsnrtowcs_r (_REENT, dst, src, (size_t) -1, len, ps);
}

#endif /* !_REENT_ONLY */
