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
 * @brief memmove() implementation.
 */

#include <sys/types.h>

/**
 * @brief Used for optimal copy speed.
 * 
 * @note sizeof(word) must be a power of two so that wmask is all ones.
 */
typedef	int word;

/**
 * @brief Word size.
 */
#define	wsize sizeof(word)

/**
 * @brief Word mask.
 */
#define	wmask (wsize - 1)

/**
 * @brief Copies bytes in memory with overlapping areas.
 * 
 * @details Copies @p n bytes from the object pointed to by @p s2 into the
 *          object pointed to by @p s1. Copying takes place as if the @p n bytes
 *          from the object pointed to by @p s2 are first copied into a
 *          temporary array of @p n bytes that does not overlap the objects
 *          pointed to by @p s1 and @p s2, and then the @p n bytes from the
 *          temporary array are copied into the object pointed to by @p s1. 
 * 
 * @returns @p s1 is returned.
 */
void *memmove(void *s1, const void *s2, size_t n)
{
	char *dst = s1;
	const char *src = s2;
	size_t t;

	/* nothing to do */
	if (n == 0 || dst == src)
		return (s1);

	/*
	 * Macros: loop-t-times; and loop-t-times, t>0
	 */
	#define	TLOOP(s) if (t) TLOOP1(s)
	#define	TLOOP1(s) do { s; } while (--t)

	/*
	 * Copy forward.
	 */
	if ((unsigned)dst < (unsigned)src)
	{
		
		/*
		 * Try to align operands. This cannot be done
		 * unless the low bits match.
		 */
		t = (int)src;
		if ((t | (int)dst) & wmask)
		{
			
			if ((t ^ (int)dst) & wmask || n < wsize)
				t = n;
			else
				t = wsize - (t & wmask);
			n -= t;
			TLOOP1(*dst++ = *src++);
		}
		
		/* Copy whole words, then mop up any trailing bytes. */
		t = n / wsize;
		TLOOP(*(word *)dst = *(word *)src; src += wsize; dst += wsize);
		t = n & wmask;
		TLOOP(*dst++ = *src++);
	}
	
	/*
	 * Copy backwards.  Otherwise essentially the same.
	 * Alignment works as before, except that it takes
	 * (t&wmask) bytes to align, not wsize-(t&wmask).
	 */
	else
	{
		src += n;
		dst += n;
		t = (int)src;
		if ((t | (int)dst) & wmask)
		{
			if ((t ^ (int)dst) & wmask || n <= wsize)
				t = n;
			else
				t &= wmask;
			n -= t;
			TLOOP1(*--dst = *--src);
		}
		
		t = n / wsize;
		TLOOP(src -= wsize; dst -= wsize; *(word *)dst = *(word *)src);
		t = n & wmask;
		TLOOP(*--dst = *--src);
	}

	return (s1);
}
