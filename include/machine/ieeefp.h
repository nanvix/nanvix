/*
 * Copyright(C) 2017 Davidson Francis <davidsondfgl@gmail.com>
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
 * Copyright (c) 1994-2009  Red Hat, Inc. All rights reserved.
 *
 * This copyrighted material is made available to anyone wishing to use,
 * modify, copy, or redistribute it subject to the terms and conditions
 * of the BSD License.   This program is distributed in the hope that
 * it will be useful, but WITHOUT ANY WARRANTY expressed or implied,
 * including the implied warranties of MERCHANTABILITY or FITNESS FOR
 * A PARTICULAR PURPOSE.  A copy of this license is available at 
 * http://www.opensource.org/licenses. Any Red Hat trademarks that are
 * incorporated in the source code or documentation are not subject to
 * the BSD License and may only be used or replicated with the express
 * permission of Red Hat, Inc.
 */

/*
 * Copyright (c) 1981-2000 The Regents of the University of California.
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without modification,
 * are permitted provided that the following conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright notice, 
 *       this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright notice,
 *       this list of conditions and the following disclaimer in the documentation
 *       and/or other materials provided with the distribution.
 *     * Neither the name of the University nor the names of its contributors 
 *       may be used to endorse or promote products derived from this software 
 *       without specific prior written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
 * AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE IMPLIED
 * WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE ARE DISCLAIMED.
 * IN NO EVENT SHALL THE COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT,
 * INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING, BUT
 * NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS OF USE, DATA, OR
 * PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND ON ANY THEORY OF LIABILITY,
 * WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE)
 * ARISING IN ANY WAY OUT OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 */

#ifndef __IEEE_BIG_ENDIAN
#ifndef __IEEE_LITTLE_ENDIAN

/* This file can define macros to choose variations of the IEEE float
   format:

   _FLT_LARGEST_EXPONENT_IS_NORMAL

	Defined if the float format uses the largest exponent for finite
	numbers rather than NaN and infinity representations.  Such a
	format cannot represent NaNs or infinities at all, but it's FLT_MAX
	is twice the IEEE value.

   _FLT_NO_DENORMALS

	Defined if the float format does not support IEEE denormals.  Every
	float with a zero exponent is taken to be a zero representation.
 
   ??? At the moment, there are no equivalent macros above for doubles and
   the macros are not fully supported by --enable-newlib-hw-fp.

   __IEEE_BIG_ENDIAN

        Defined if the float format is big endian.  This is mutually exclusive
        with __IEEE_LITTLE_ENDIAN.

   __IEEE_LITTLE_ENDIAN
 
        Defined if the float format is little endian.  This is mutually exclusive
        with __IEEE_BIG_ENDIAN.

   Note that one of __IEEE_BIG_ENDIAN or __IEEE_LITTLE_ENDIAN must be specified for a
   platform or error will occur.

   __IEEE_BYTES_LITTLE_ENDIAN

        This flag is used in conjunction with __IEEE_BIG_ENDIAN to describe a situation 
	whereby multiple words of an IEEE floating point are in big endian order, but the
	words themselves are little endian with respect to the bytes.

   _DOUBLE_IS_32BITS 

        This is used on platforms that support double by using the 32-bit IEEE
        float type.

   _FLOAT_ARG

        This represents what type a float arg is passed as.  It is used when the type is
        not promoted to double.
	
*/

#if (defined(__arm__) || defined(__thumb__)) && !defined(__MAVERICK__)
/* ARM traditionally used big-endian words; and within those words the
   byte ordering was big or little endian depending upon the target.
   Modern floating-point formats are naturally ordered; in this case
   __VFP_FP__ will be defined, even if soft-float.  */
#ifdef __VFP_FP__
# ifdef __ARMEL__
#  define __IEEE_LITTLE_ENDIAN
# else
#  define __IEEE_BIG_ENDIAN
# endif
#else
# define __IEEE_BIG_ENDIAN
# ifdef __ARMEL__
#  define __IEEE_BYTES_LITTLE_ENDIAN
# endif
#endif
#endif

#if defined (__aarch64__)
#if defined (__AARCH64EL__)
#define __IEEE_LITTLE_ENDIAN
#else
#define __IEEE_BIG_ENDIAN
#endif
#endif

#ifdef __epiphany__
#define __IEEE_LITTLE_ENDIAN
#define Sudden_Underflow 1
#endif

#ifdef __hppa__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __nds32__
#ifdef __big_endian__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __SPU__
#define __IEEE_BIG_ENDIAN

#define isfinite(__y) \
	(__extension__ ({int __cy; \
		(sizeof (__y) == sizeof (float))  ? (1) : \
		(__cy = fpclassify(__y)) != FP_INFINITE && __cy != FP_NAN;}))

#define isinf(__x) ((sizeof (__x) == sizeof (float))  ?  (0) : __isinfd(__x))
#define isnan(__x) ((sizeof (__x) == sizeof (float))  ?  (0) : __isnand(__x))

/*
 * Macros for use in ieeefp.h. We can't just define the real ones here
 * (like those above) as we have name space issues when this is *not*
 * included via generic the ieeefp.h.
 */
#define __ieeefp_isnanf(x)	0
#define __ieeefp_isinff(x)	0
#define __ieeefp_finitef(x)	1
#endif

#ifdef __sparc__
#ifdef __LITTLE_ENDIAN_DATA__
#define __IEEE_LITTLE_ENDIAN
#else
#define __IEEE_BIG_ENDIAN
#endif
#endif

#if defined(__m68k__) || defined(__mc68000__)
#define __IEEE_BIG_ENDIAN
#endif

#if defined(__mc68hc11__) || defined(__mc68hc12__) || defined(__mc68hc1x__)
#define __IEEE_BIG_ENDIAN
#ifdef __HAVE_SHORT_DOUBLE__
# define _DOUBLE_IS_32BITS
#endif
#endif

#if defined (__H8300__) || defined (__H8300H__) || defined (__H8300S__) || defined (__H8500__) || defined (__H8300SX__)
#define __IEEE_BIG_ENDIAN
#define _FLOAT_ARG float
#define _DOUBLE_IS_32BITS
#endif

#if defined (__xc16x__) || defined (__xc16xL__) || defined (__xc16xS__)
#define __IEEE_LITTLE_ENDIAN
#define _FLOAT_ARG float
#define _DOUBLE_IS_32BITS
#endif


#ifdef __sh__
#ifdef __LITTLE_ENDIAN__
#define __IEEE_LITTLE_ENDIAN
#else
#define __IEEE_BIG_ENDIAN
#endif
#if defined(__SH2E__) || defined(__SH3E__) || defined(__SH4_SINGLE_ONLY__) || defined(__SH2A_SINGLE_ONLY__)
#define _DOUBLE_IS_32BITS
#endif
#endif

#ifdef _AM29K
#define __IEEE_BIG_ENDIAN
#endif

#ifdef _WIN32
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __i386__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __i960__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __lm32__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __M32R__
#define __IEEE_BIG_ENDIAN
#endif

#if defined(_C4x) || defined(_C3x)
#define __IEEE_BIG_ENDIAN
#define _DOUBLE_IS_32BITS
#endif

#ifdef __TMS320C6X__
#ifdef _BIG_ENDIAN
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __TIC80__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __MIPSEL__
#define __IEEE_LITTLE_ENDIAN
#endif
#ifdef __MIPSEB__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __MMIX__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __D30V__
#define __IEEE_BIG_ENDIAN
#endif

/* necv70 was __IEEE_LITTLE_ENDIAN. */

#ifdef __W65__
#define __IEEE_LITTLE_ENDIAN
#define _DOUBLE_IS_32BITS
#endif

#if defined(__Z8001__) || defined(__Z8002__)
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __m88k__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __mn10300__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __mn10200__
#define __IEEE_LITTLE_ENDIAN
#define _DOUBLE_IS_32BITS
#endif

#ifdef __v800
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __v850
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __D10V__
#define __IEEE_BIG_ENDIAN
#if __DOUBLE__ == 32
#define _DOUBLE_IS_32BITS
#endif
#endif

#ifdef __PPC__
#if (defined(_BIG_ENDIAN) && _BIG_ENDIAN) || (defined(_AIX) && _AIX)
#define __IEEE_BIG_ENDIAN
#else
#if (defined(_LITTLE_ENDIAN) && _LITTLE_ENDIAN) || (defined(__sun__) && __sun__) || (defined(_WIN32) && _WIN32)
#define __IEEE_LITTLE_ENDIAN
#endif
#endif
#endif

#ifdef __xstormy16__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __arc__
#ifdef __big_endian__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __CRX__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __fr30__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __mcore__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __mt__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __frv__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __moxie__
#ifdef __MOXIE_BIG_ENDIAN__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __ia64__
#ifdef __BIG_ENDIAN__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __AVR__
#define __IEEE_LITTLE_ENDIAN
#define _DOUBLE_IS_32BITS
#endif

#if defined(__or1k__) || defined(__OR1K__) || defined(__OR1KND__)
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __IP2K__
#define __IEEE_BIG_ENDIAN
#define __SMALL_BITFIELDS
#define _DOUBLE_IS_32BITS
#endif

#ifdef __iq2000__
#define __IEEE_BIG_ENDIAN
#endif

#ifdef __MAVERICK__
#ifdef __ARMEL__
#  define __IEEE_LITTLE_ENDIAN
#else  /* must be __ARMEB__ */
#  define __IEEE_BIG_ENDIAN
#endif /* __ARMEL__ */
#endif /* __MAVERICK__ */

#ifdef __m32c__
#define __IEEE_LITTLE_ENDIAN
#define __SMALL_BITFIELDS
#endif

#ifdef __CRIS__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __BFIN__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __x86_64__
#define __IEEE_LITTLE_ENDIAN
#endif

#ifdef __mep__
#ifdef __LITTLE_ENDIAN__
#define __IEEE_LITTLE_ENDIAN
#else
#define __IEEE_BIG_ENDIAN
#endif
#endif

#ifdef __MICROBLAZE__
#ifndef __MICROBLAZEEL__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif
#endif

#ifdef __MSP430__
#define __IEEE_LITTLE_ENDIAN
#define __SMALL_BITFIELDS	/* 16 Bit INT */
#endif

#ifdef __RL78__
#define __IEEE_LITTLE_ENDIAN
#define __SMALL_BITFIELDS	/* 16 Bit INT */
#ifndef __RL78_64BIT_DOUBLES__
#define _DOUBLE_IS_32BITS
#endif
#endif

#ifdef __RX__

#ifdef __RX_BIG_ENDIAN__
#define __IEEE_BIG_ENDIAN
#else
#define __IEEE_LITTLE_ENDIAN
#endif

#ifndef __RX_64BIT_DOUBLES__
#define _DOUBLE_IS_32BITS
#endif

#ifdef __RX_16BIT_INTS__
#define __SMALL_BITFIELDS
#endif

#endif

#if (defined(__CR16__) || defined(__CR16C__) ||defined(__CR16CP__))
#define __IEEE_LITTLE_ENDIAN
#define __SMALL_BITFIELDS	/* 16 Bit INT */
#endif

#ifdef __NIOS2__
# ifdef __nios2_big_endian__
#  define __IEEE_BIG_ENDIAN
# else
#  define __IEEE_LITTLE_ENDIAN
# endif
#endif

#ifndef __IEEE_BIG_ENDIAN
#ifndef __IEEE_LITTLE_ENDIAN
#error Endianess not declared!!
#endif /* not __IEEE_LITTLE_ENDIAN */
#endif /* not __IEEE_BIG_ENDIAN */

#endif /* not __IEEE_LITTLE_ENDIAN */
#endif /* not __IEEE_BIG_ENDIAN */

