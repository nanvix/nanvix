/* 
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * This file is part of Nanvix.
 * 
 * Nanvix is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 * 
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix.  If not, see <http://www.gnu.org/licenses/>.
 */

/*
 * Copyright (c) 1990 Regents of the University of California.
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

/**
 * @file
 * 
 * @brief Character types library.
 */

#ifndef _CTYPE_H_
#define _CTYPE_H_

	#ifdef _POSIX_C_SOURCE
		#include <locale.h>
	#endif /* _POSIX_C_SOURCE */

	/* Character types. */
	#define	_U	0x01 /* Upper-case character.         */
	#define	_L	0x02 /* Lower-case character.         */
	#define	_N	0x04 /* Decimal number character.     */
	#define	_S	0x08 /* White-space character.        */
	#define	_P	0x10 /* Punctuation character.        */
	#define	_C	0x20 /* Control character.            */
	#define	_X	0x40 /* Hexadecimal number character. */
	#define	_B	0x80 /* Blank space character.        */
	
	/**
	 * @defgroup ctype ctype.h
	 * 
	 * @brief Character types library.
	 * 
	 * @todo Do not ignore current locale.
	 * @todo Implement all extensions to the ISO C standard.
	 */
	/**@{*/
	
	/* Forward definitions. */
	extern int isalnum(int);
	extern int isalpha(int);
	extern int isblank(int);
	extern int iscntrl(int);
	extern int isdigit(int);
	extern int isgraph(int);
	extern int islower(int);
	extern int isprint(int);
	extern int ispunct(int);
	extern int isspace(int);
	extern int isupper(int);
	extern int isxdigit(int);
	extern int tolower(int);
	extern int toupper(int);
	
#ifdef _POSIX_C_SOURCE
	
	/* Forward definitions. */
	extern int isalnum_l(int, locale_t);
	extern int isalpha_l(int, locale_t);
	extern int isblank_l(int, locale_t);
	extern int iscntrl_l(int, locale_t);
	extern int isdigit_l(int, locale_t);
	extern int isgraph_l(int, locale_t);
	extern int islower_l(int, locale_t);
	extern int isprint_l(int, locale_t);
	extern int ispunct_l(int, locale_t);
	extern int isspace_l(int, locale_t);
	extern int isupper_l(int, locale_t);
	extern int isxdigit_l(int, locale_t);
	extern int tolower_l(int, locale_t);
	extern int toupper_l(int, locale_t);

#endif /* _POSIX_C_SOURCE */
	
	/**@}*/
	
	/* Forward definitions. */
	extern const unsigned char _ctype[];
	extern char _maplower[];
	extern char _mapupper[];

#endif /* _CTYPE_H_ */
