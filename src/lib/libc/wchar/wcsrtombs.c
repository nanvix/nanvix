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

size_t _wcsrtombs_r(struct _reent *r, char *dst, const wchar_t **src,
	size_t len, mbstate_t *ps)
{
  return _wcsnrtombs_r (r, dst, src, (size_t) -1, len, ps);
} 

#ifndef _REENT_ONLY

/**
 * @brief Converts a wide-character string to a character string (restartable).
 *
 * @details Converts a sequence of wide characters from the array
 * indirectly pointed to by @p src into a sequence of corresponding
 * characters, beginning in the conversion state described by the
 * object pointed to by @p ps. If @p dst is not a null pointer, the 
 * converted characters shall then be stored into the array pointed
 * to by @p dst.
 *
 * @return If conversion stops because a code is reached that does not
 * correspond to a valid character, an encoding error occurs. In this
 * case, the function stores the value of the macro [EILSEQ] in
 * errno and return (size_t)-1; the conversion state is undefined. 
 * Otherwise, the function returns the number of bytes in the
 * resulting character sequence, not including the terminating null (if any).
 */
size_t wcsrtombs(char *restrict dst, const wchar_t **restrict src, size_t len,
	mbstate_t *restrict ps)
{
  return _wcsnrtombs_r (_REENT, dst, src, (size_t) -1, len, ps);
}

#endif /* !_REENT_ONLY */
