/*
 * Copyright(C) 2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
#include <ctype.h>

/**
 * @brief Tests for a white-space character. 
 *
 * @details Tests whether @p c is a character of class space in the
 * current locale. The @p c argument is an int, the value of which the
 * application shall ensure is representable as an unsigned char or
 * equal to the value of the macro #EOF. If the argument has any other
 * value, the behavior is undefined.
 *
 * @param Character to test.
 *
 * @returns Returns non-zero if @p c is a white-space character;
 * otherwise, it returns 0.
 */
int isspace(int c)
{
	return(__ctype_ptr__[c+1] & _S);
}

