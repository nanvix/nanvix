/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
 *                   Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef CTYPE_H_
#define CTYPE_H_

	#include <_ansi.h>

	#define _U  01
	#define _L  02
	#define _N  04
	#define _S  010
	#define _P  020
	#define _C  040
	#define _X  0100
	#define _B  0200

	/* Forward definitions. */
	extern int isalnum(int);
	extern int isalpha(int);
	extern int isascii(int);
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

	/* Forward Definitions. */
	const char *__ctype_ptr__;

#ifndef _XOPEN_SOURCE

	/* Forward definitions. */
	extern int toascii(int);

#endif

#endif /* CTYPE_H_ */
