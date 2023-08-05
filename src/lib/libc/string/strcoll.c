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
 * @brief strcoll() implementation.
 */

#include <string.h>

/**
 * @brief String comparison using collating information.
 *
 * @param s1 Pointer to first string.
 * @param s2 Pointer to second string.
 *
 * @returns An integer greater than, equal to, or less than 0, according to
 *          whether the string pointed to by @p s1 is greater than, equal to,
 *          or less than the string pointed to by @p s2 when both are
 *          interpreted as appropriate to the current locale.
 *
 * @note On error errno may be set, but no return value is reserved to indicate
 *       an error.
 *
 * @todo Perform comparison using collating information.
 *
 * @version IEEE Std 1003.1, 2013 Edition
 */
int strcoll(const char *s1, const char *s2)
{
	return (strcmp(s1, s2));
}
