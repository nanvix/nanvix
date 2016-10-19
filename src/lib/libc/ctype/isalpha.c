/*
 * Copyright(C) 2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
FUNCTION
	<<isalpha>>---alphabetic character predicate

INDEX
	isalpha

ANSI_SYNOPSIS
	#include <ctype.h>
	int isalpha(int <[c]>);

TRAD_SYNOPSIS
	#include <ctype.h>
	int isalpha(<[c]>);

DESCRIPTION
<<isalpha>> is a macro which classifies ASCII integer values by table
lookup.  It is a predicate returning non-zero when <[c]> represents an
alphabetic ASCII character, and 0 otherwise.  It is defined only if
<[c]> is representable as an unsigned char or if <[c]> is EOF.

You can use a compiled subroutine instead of the macro definition by
undefining the macro using `<<#undef isalpha>>'.

RETURNS
<<isalpha>> returns non-zero if <[c]> is a letter (<<A>>--<<Z>> or
<<a>>--<<z>>). 

PORTABILITY
<<isalpha>> is ANSI C.

No supporting OS subroutines are required.
*/

#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(isalpha,(c),int c)
{
	return(__ctype_ptr__[c+1] & (_U|_L));
}

