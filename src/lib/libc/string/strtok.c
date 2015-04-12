/*
 * Copyright (c) 1990 The Regents of the University of California.
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
 * 3. All advertising materials mentioning features or use of this software
 *    must display the following acknowledgement:
 *	This product includes software developed by the University of
 *	California, Berkeley and its contributors.
 * 4. Neither the name of the University nor the names of its contributors
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

/**
 * @file
 * 
 * @brief strtok() implementation.
 */
 
#include <stdlib.h>

/**
 * @brief Scan point.
 */
static char *_scanpoint = NULL;

/**
 * @brief Splits string into tokens.
 * 
 * @details Breaks the string pointed to by @p s1 into a sequence of tokens,
 *          each of which is delimited by a byte from the string pointed to by
 *          @p s2. The first call in the sequence has @p s1 as its first
 *          argument, and is followed by calls with a null pointer as their
 *          first argument. The separator string pointed to by @p s2 may be
 *          different from call to call.
 * 
 *          The first call in the sequence searches the string pointed to by
 *          @p s1 for the first byte that is not contained in the current
 *          separator string pointed to by @p s2. If no such byte is found,
 *          then there are no tokens in the string pointed to by @p s1 and
 *          strtok() returns a null pointer. If such a byte is found, it is the
 *          start of the first token. 
 * 
 *          The strtok() function then searches from there for a byte that is
 *          contained in the current separator string. If no such byte is
 *          found, the current token extends to the end of the string pointed to
 *          by @p s1, and subsequent searches for a token will return a null
 *          pointer. If such a byte is found, it is overwritten by a null byte,
 *          which terminates the current token. The strtok() function saves a
 *          pointer to the following byte, from which the next search for a
 *          token will start.
 * 
 *          Each subsequent call, with a null pointer as the value of the first
 *          argument, starts searching from the saved pointer and behaves as
 *          described above.
 * 
 * @returns A pointer to the first byte of a token. Otherwise, if there is no
 *          token, strtok() returns a null pointer. 
 */
char *strtok(char *s1, const char *s2)
{
	char *scan;
	char *tok;
	const char *dscan;

	if (s1 == NULL && _scanpoint == NULL)
      return (NULL);
      
	if (s1 != NULL)
		scan = s1;
	else
		scan = _scanpoint;

	/*
	 * Scan leading delimiters.
	 */
	for (/* noop*/ ; *scan != '\0'; scan++)
	{
		for (dscan = s2; *dscan != '\0'; dscan++)
		{
			if (*scan == *dscan)
				break;
		}

		if (*dscan == '\0')
			break;
	}
	if (*scan == '\0')
	{
		_scanpoint = NULL;
		return (NULL);
    }

	tok = scan;

	/*
	 * Scan token.
	 */
	for (/* noop */; *scan != '\0'; scan++)
	{
		/* ++ moved down. */
		for (dscan = s2; *dscan != '\0';)
		{
			if (*scan == *dscan++)
			{
				_scanpoint = scan + 1;
				*scan = '\0';
				return (tok);
			}
		}
	}

	/* Reached end of string. */
	_scanpoint = NULL;
	
	return (tok);
}
