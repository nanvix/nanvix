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
	<<isblank>>---blank character predicate

INDEX
	isblank

ANSI_SYNOPSIS
	#include <ctype.h>
	int isblank(int <[c]>);

TRAD_SYNOPSIS
	#include <ctype.h>
	int isblank(<[c]>);

DESCRIPTION
<<isblank>> is a function which classifies ASCII integer values by table
lookup.  It is a predicate returning non-zero for blank characters, and 0
for other characters.  It is defined only if <[c]> is representable as an
unsigned char or if <[c]> is EOF.

RETURNS
<<isblank>> returns non-zero if <[c]> is a blank character.

PORTABILITY
<<isblank>> is C99.

No supporting OS subroutines are required.
*/

#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(isblank,(c),int c)
{
	return ((__ctype_ptr__[c+1] & _B) || (c == '\t'));
}
