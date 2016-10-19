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
<<isupper>>---uppercase character predicate

INDEX
isupper

ANSI_SYNOPSIS
#include <ctype.h>
int isupper(int <[c]>);

TRAD_SYNOPSIS
#include <ctype.h>
int isupper(<[c]>);

DESCRIPTION
<<isupper>> is a macro which classifies ASCII integer values by table
lookup.  It is a predicate returning non-zero for uppercase letters
(<<A>>--<<Z>>), and 0 for other characters.  It is defined only when
<<isascii>>(<[c]>) is true or <[c]> is EOF.

You can use a compiled subroutine instead of the macro definition by
undefining the macro using `<<#undef isupper>>'.

RETURNS
<<isupper>> returns non-zero if <[c]> is a uppercase letter (A-Z).

PORTABILITY
<<isupper>> is ANSI C.

No supporting OS subroutines are required.
*/
#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(isupper,(c),int c)
{
	return ((__ctype_ptr__[c+1] & (_U|_L)) == _U);
}

