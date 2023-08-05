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

/**
 * @file
 *
 * @brief strnlen() implementation.
 */

#include <sys/types.h>

/**
 * @brief Gets the length of a fixed-size string.
 *
 * @details Computes the smaller of the number of bytes in the array to which
 *          @p str points, not including the terminating null character, or the
 *          value of the @p maxlen argument.
 *
 * @returns An integer containing the smaller of either the length of the string
 *          pointed to by @p str or @p maxlen.
 */
size_t strnlen(const char *str, size_t maxlen)
{
	const char *p;

	/* Count the number of characters. */
	for (p = str; *p != '\0' && maxlen > 0; p++, maxlen--)
		/* No operation.*/;

	return (p - str);
}
