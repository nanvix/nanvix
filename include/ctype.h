/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef _CTYPE_H_
#define _CTYPE_H_

#include <_ansi.h>

#define _U  01
#define _L  02
#define _N  04
#define _S  010
#define _P  020
#define _C  040
#define _X  0100
#define _B  0200

extern int isalnum(int __c);
extern int isalpha(int __c);
extern int isascii(int __c);
extern int isblank(int __c);
extern int iscntrl(int __c);
extern int isdigit(int __c);
extern int isgraph(int __c);
extern int islower(int __c);
extern int isprint(int __c);
extern int ispunct(int __c);
extern int isspace(int __c);
extern int isupper(int __c);
extern int isxdigit(int __c);
extern int tolower(int __c);
extern int toupper(int __c);

const char *__ctype_ptr__;

/* 
  These macros are intentionally written in a manner that will trigger
  a gcc -Wall warning if the user mistakenly passes a 'char' instead
  of an int containing an 'unsigned char'.  Note that the sizeof will
  always be 1, which is what we want for mapping EOF to __ctype_ptr__[0];
  the use of a raw index inside the sizeof triggers the gcc warning if
  __c was of type char, and sizeof masks side effects of the extra __c.
  Meanwhile, the real index to __ctype_ptr__+1 must be cast to int,
  since isalpha(0x100000001LL) must equal isalpha(1), rather than being
  an out-of-bounds reference on a 64-bit machine.  
*/
#define __ctype_lookup(__c) ((__ctype_ptr__+sizeof(""[__c]))[(int)(__c)])

#define isalpha(__c)  (__ctype_lookup(__c)&(_U|_L))
#define isascii(__c)  ((unsigned)(__c)<=0177)
#define isupper(__c)  ((__ctype_lookup(__c)&(_U|_L))==_U)
#define islower(__c)  ((__ctype_lookup(__c)&(_U|_L))==_L)
#define isdigit(__c)  (__ctype_lookup(__c)&_N)
#define isxdigit(__c) (__ctype_lookup(__c)&(_X|_N))
#define isspace(__c)  (__ctype_lookup(__c)&_S)
#define ispunct(__c)  (__ctype_lookup(__c)&_P)
#define isalnum(__c)  (__ctype_lookup(__c)&(_U|_L|_N))
#define isprint(__c)  (__ctype_lookup(__c)&(_P|_U|_L|_N|_B))
#define isgraph(__c)  (__ctype_lookup(__c)&(_P|_U|_L|_N))
#define iscntrl(__c)  (__ctype_lookup(__c)&_C)


#ifndef __STRICT_ANSI__
extern int toascii(int __c);
#define _tolower(__c) ((unsigned char)(__c) - 'A' + 'a')
#define _toupper(__c) ((unsigned char)(__c) - 'a' + 'A')
#define toascii(__c)  ((__c)&0177)
#endif

#endif /* _CTYPE_H_ */
