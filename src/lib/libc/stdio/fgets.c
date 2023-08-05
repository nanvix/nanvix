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

#include <errno.h>
#include <stdlib.h>
#include <stdio.h>

/*
 * Reads a string from a file.
 */
char *fgets(char *str, int n, FILE *stream)
{
	int c = EOF;   /* Working character. */
	char *p = str; /* Write pointer.     */

	/* Read string. */
	while ((--n > 0) && ((c = getc(stream)) != EOF))
	{
		if (c == '\n')
			break;
		*p++ = c;
	}

	if ((c == EOF) && (p == str))
		return (NULL);

	*p = '\0';

	return (str);
}
