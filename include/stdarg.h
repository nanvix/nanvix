/*
 * Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/* Copyright (C) 1989-2015 Free Software Foundation, Inc.

This file is part of GCC.

GCC is free software; you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation; either version 3, or (at your option)
any later version.

GCC is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

Under Section 7 of GPL version 3, you are granted additional
permissions described in the GCC Runtime Library Exception, version
3.1, as published by the Free Software Foundation.

You should have received a copy of the GNU General Public License and
a copy of the GCC Runtime Library Exception along with this program;
see the files COPYING3 and COPYING.RUNTIME respectively.  If not, see
<http://www.gnu.org/licenses/>.  */

#ifndef STDARG_H_
#define STDARG_H_

#ifdef i386

    typedef char* va_list;

    #define __va_rounded_size(TYPE)  \
      (((sizeof (TYPE) + sizeof (int) - 1) / sizeof (int)) * sizeof (int))

    #define va_start(AP, LASTARG) \
        (AP = ((char *) &(LASTARG) + __va_rounded_size (LASTARG)))

    void va_end (va_list);
    #define va_end(AP)

    #define va_arg(AP, TYPE)                        \
      (AP += __va_rounded_size (TYPE),              \
      *((TYPE *) (AP - __va_rounded_size (TYPE))))  \

    #ifdef __need___va_list
    #ifndef __VALIST
    #define __VALIST char*
    #endif
    #endif

#elif or1k

	#define va_start(v,l) __builtin_va_start(v,l)
	#define va_end(v)     __builtin_va_end(v)
	#define va_arg(v,l)   __builtin_va_arg(v,l)

	typedef __builtin_va_list va_list; 

#endif

#endif /* STDARG_H_ */
