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

#include <stdio.h>
#include <stdarg.h>

/*
 * Writes format output of a stdarg argument list to a file.
 */
int vfprintf(FILE *stream, const char *format, va_list ap)
{
	int n;             /* Characters written. */
	char buffer[1024]; /* Buffer.             */

	/* Format string. */
	n = vsprintf(buffer, format, ap);

	/* Write formated string to file. */
	if (n > 0)
		fputs(buffer, stream);

	return (n);
}

