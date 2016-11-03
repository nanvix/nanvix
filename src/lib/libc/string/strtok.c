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

/* undef STRICT_ANSI so that strtok_r prototype will be defined */
#undef  __STRICT_ANSI__
#include <string.h>
#include <_ansi.h>
#include <reent.h>

#ifndef _REENT_ONLY

extern char *__strtok_r (char *, const char *, char **, int);

/**
 * @brief Splits string into tokens.
 *
 * @details Breaks the string pointed to by @p s into a sequence
 * of tokens, each of which is delimited by a byte from the string 
 * pointed to by @p delim. The first call in the sequence has @p s
 * as its first argument, and is followed by calls with a null pointer
 * as their first argument. The separator string pointed to by @p delim
 * may be different from call to call.
 *
 * @return Returns a pointer to the first byte of a token. Otherwise, if
 * there is no token, returns a null pointer.
 */
char *strtok(register char *restrict s, register const char *restrict delim)
{
	struct _reent *reent = _REENT;

	_REENT_CHECK_MISC(reent);
	return __strtok_r (s, delim, &(_REENT_STRTOK_LAST(reent)), 1);
}
#endif
