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
#include <wchar.h>

/**
 * @brief Wide-character string transformation.
 *
 * @details Transforms the wide-character string pointed to
 * by @p b and place the resulting wide-character string into
 * the array pointed to by @p a.
 *
 * @return Returns the length of the transformed wide-character
 * string (not including the terminating null wide-character code).
 */
size_t wcsxfrm(wchar_t *restrict a, const wchar_t *restrict b, size_t n)
{
  return wcslcpy (a, b, n);
}
