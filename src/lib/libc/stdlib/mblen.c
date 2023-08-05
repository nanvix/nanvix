/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @param s Multi-byte character.
 * @param n Maximum number of bytes to consider.
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
