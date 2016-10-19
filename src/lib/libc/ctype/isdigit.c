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
<<isdigit>>---decimal digit predicate

INDEX
isdigit

ANSI_SYNOPSIS
#include <ctype.h>
int isdigit(int <[c]>);

TRAD_SYNOPSIS
#include <ctype.h>
int isdigit(<[c]>);

DESCRIPTION
<<isdigit>> is a macro which classifies ASCII integer values by table
lookup.  It is a predicate returning non-zero for decimal digits, and 0 for
other characters.  It is defined only if <[c]> is representable as an
unsigned char or if <[c]> is EOF.

You can use a compiled subroutine instead of the macro definition by
undefining the macro using `<<#undef isdigit>>'.

RETURNS
<<isdigit>> returns non-zero if <[c]> is a decimal digit (<<0>>--<<9>>).

PORTABILITY
<<isdigit>> is ANSI C.

No supporting OS subroutines are required.
*/

#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(isdigit,(c),int c)
{
	return(__ctype_ptr__[c+1] & _N);
}
