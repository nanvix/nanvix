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
	<<toascii>>---force integers to ASCII range

INDEX
	toascii

ANSI_SYNOPSIS
	#include <ctype.h>
	int toascii(int <[c]>);

TRAD_SYNOPSIS
	#include <ctype.h>
	int toascii(<[c]>);
	int (<[c]>);

DESCRIPTION
<<toascii>> is a macro which coerces integers to the ASCII range (0--127) by zeroing any higher-order bits.

You can use a compiled subroutine instead of the macro definition by
undefining this macro using `<<#undef toascii>>'.

RETURNS
<<toascii>> returns integers between 0 and 127.

PORTABILITY
<<toascii>> is not ANSI C.

No supporting OS subroutines are required.
*/
typedef int toascii_avoids_empty_translation_unit;

#ifdef _XOPEN_SOURCE

#include <_ansi.h>
#include <ctype.h>

int
_DEFUN(toascii,(c),int c)
{
  return (c)&0177;
}

#endif /* _XOPEN_SOURCE */
