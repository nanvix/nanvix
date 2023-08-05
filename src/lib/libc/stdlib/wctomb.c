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
 * @brief wctomb() implementation.
 */

#include <stdlib.h>

/**
 * @brief Cconverts a wide-character code to a character.
 *
 * @param s     Store location.
 * @param wchar Wide-character to convert.
 *
 * @returns If @p s is a null pointer,a non-zero or 0 value is returns, if
 *          character encodings, respectively, do or do not have state-dependent
 *          encodings. If @p s is not a null pointer, -1 is returned if the
 *          value of @p wchar does not correspond to a valid character, or
 *          return the number of bytes that constitute the character
 *          corresponding to the value of @p wchar.
 */
int wctomb(char *s, wchar_t wchar)
{
	if (s != NULL)
    {
		*s = wchar;
		return (1);
    }

	return (0);
}
