/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * Copyright (C) 1991-1996 Free Software Foundation, Inc.
 * 
 * This file is part of the GNU C Library.
 * 
 * The GNU C Library free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 * 
 * The GNU C Library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston,
 * MA 02110-1301, USA.
 */

/**
 * @file
 * 
 * @brief mblen() implementation.
 */
 
#include <stdlib.h>

/**
 * @brief Gets number of bytes in a character.
 * 
 * @details If @p s is not a null pointer, mblen() determines the number of
 *          bytes constituting the character pointed to by @p s.
 * 
 *          The behavior of this function is affected by the LC_CTYPE category
 *          of the current locale. For a state-dependent encoding, this function
 *          is placed into its initial state by a call for which its character
 *          pointer argument, @p s, is a null pointer. Subsequent calls with
 *          @p s as other than a null pointer cause the internal state of the
 *          function to be altered as necessary. A call with @p s as a null
 *          pointer cause this function to return a non-zero value if encodings
 *          have state dependency, and 0 otherwise. If the implementation
 *          employs special bytes to change the shift state, these bytes do
 *          not produce separate wide-character codes, but are grouped with an
 *          adjacent character. Changing the LC_CTYPE category causes the shift
 *          state of this function to be unspecified.
 * 
 * @returns If s is a null pointer, mblen() returns a non-zero or 0 value, if
 *          character encodings, respectively, do or do not have state-dependent
 *          encodings. If s is not a null pointer, mblen() either returns 0
 *          (if s points to the null byte), or returns the number of bytes that
 *          constitute the character (if the next n or fewer bytes form a valid
 *          character), or return -1 (if they do not form a valid character) and
 *          is set errno to indicate the error. In no case shall the value
 *          returned be greater than @p n or the value of the MB_CUR_MAX macro.
 * 
 * @note The mblen() function is not thread-safe.
 */
int mblen(const char *s, size_t n)
{
	return(mbtowc((wchar_t *) NULL, s, n));
}
