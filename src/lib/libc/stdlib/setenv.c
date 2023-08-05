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
 * Copyright (c) 1992 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following conditions
 * are met:
 * 1. Redistributions of source code must retain the above copyright
 *    notice, this list of conditions and the following disclaimer.
 * 2. Redistributions in binary form must reproduce the above copyright
 *    notice, this list of conditions and the following disclaimer in the
 *    documentation and/or other materials provided with the distribution.
 * 3. Neither the name of the University nor the names of its contributors
 *    may be used to endorse or promote products derived from this software
 *    without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE REGENTS AND CONTRIBUTORS ``AS IS'' AND
 * ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
 * IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
 * ARE DISCLAIMED.  IN NO EVENT SHALL THE REGENTS OR CONTRIBUTORS BE LIABLE
 * FOR ANY DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL
 * DAMAGES (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS
 * OR SERVICES; LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION)
 * HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT
 * LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY
 * OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF
 * SUCH DAMAGE.
 */

/**
 * @file
 *
 * @brief setenv() implementation.
 */

#include <unistd.h>
#include <stdlib.h>
#include <string.h>
#include "findenv.h"

/**
 * @brief Adds or change environment variable.
 *
 * @param envname   Environment variable name.
 * @param envval    Environment variable value.
 * @param overwrite Overwrite environment variable?
 *
 * @returns Upon successful completion, zero is returned. Otherwise, -1 is
 *          returned, errno set to indicate the error, and the environment
 *          is unchanged.
 *
 * @todo The setenv() function shall fail if the envname argument points to an
 *       empty string or points to a string containing an '=' character.
 */
int setenv(const char *envname, const char *envval, int overwrite)
{
	static int alloced;		/* If allocated space before. */
	register char *C;
	size_t l_envval;
	int offset;

	/* No '=' in envval. */
	if (*envval == '=')
		++envval;

	/* Find if already exists. */
	l_envval = strlen(envval);
	if ((C = findenv(envname, &offset)))
	{
		if (!overwrite)
			return (0);

		if (strlen(C) >= l_envval)
		{
			/* old larger; copy over */
			while ((*C++ = *envval++))
				/* noop */ ;

			return (0);
		}
	}

	/* Create new slot. */
	else
	{
		register int cnt;
		register char **P;

			for (P = environ, cnt = 0; *P; ++P, ++cnt)
				/* noop */;

		/* Just increase size. */
		if (alloced)
		{
			environ =
				realloc((char *) environ, (size_t) (sizeof(char *)*(cnt + 2)));

			if (environ == NULL)
				return (-1);
		}

		/* Get new space. */
		else
		{
			alloced = 1;
			P = (char **) malloc ((size_t) (sizeof (char *) * (cnt + 2)));
			if (P == NULL)
				return (-1);

			/* copy old entries into it */
			memmove((char *) P, (char *) environ, cnt * sizeof (char *));
			environ = P;
		}
				environ[cnt + 1] = NULL;
				offset = cnt;
	}

	/* No '=' in envname. */
	for (C = (char *) envname; (*C != '\0') && (*C != '='); C++)
		/* noop */ ;

	/* envname + '=' + envval */
	environ[offset] = malloc ((size_t)((int) (C - envname) + l_envval + 2));
	if (environ[offset] == NULL)
		return (-1);
	for (C = environ[offset]; (*C = *envname++) && *C != '='; ++C)
		/* noop */ ;
	for (*C++ = '='; (*C++ = *envval++); /* noop */)
		/* noop */ ;

	return (0);
}
