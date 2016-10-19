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
<<ispunct>>---punctuation character predicate

INDEX
ispunct

ANSI_SYNOPSIS
#include <ctype.h>
int ispunct(int <[c]>);

TRAD_SYNOPSIS
#include <ctype.h>
int ispunct(<[c]>);

DESCRIPTION
<<ispunct>> is a macro which classifies ASCII integer values by table
lookup.  It is a predicate returning non-zero for printable
punctuation characters, and 0 for other characters.  It is defined only
if <[c]> is representable as an unsigned char or if <[c]> is EOF.

You can use a compiled subroutine instead of the macro definition by
undefining the macro using `<<#undef ispunct>>'.

RETURNS
<<ispunct>> returns non-zero if <[c]> is a printable punctuation character 
(<<isgraph(<[c]>) && !isalnum(<[c]>)>>).

PORTABILITY
<<ispunct>> is ANSI C.

No supporting OS subroutines are required.
*/

#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(ispunct,(c),int c)
{
	return(__ctype_ptr__[c+1] & _P);
}

