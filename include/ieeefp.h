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

#ifndef _IEEE_FP_H_
#define _IEEE_FP_H_

#include "_ansi.h"

#include <machine/ieeefp.h>

_BEGIN_STD_C

/* FIXME FIXME FIXME:
   Neither of __ieee_{float,double}_shape_tape seem to be used anywhere
   except in libm/test.  If that is the case, please delete these from here.
   If that is not the case, please insert documentation here describing why
   they're needed.  */

#ifdef __IEEE_BIG_ENDIAN

typedef union 
{
  double value;
  struct 
  {
    unsigned int sign : 1;
    unsigned int exponent: 11;
    unsigned int fraction0:4;
    unsigned int fraction1:16;
    unsigned int fraction2:16;
    unsigned int fraction3:16;
    
  } number;
  struct 
  {
    unsigned int sign : 1;
    unsigned int exponent: 11;
    unsigned int quiet:1;
    unsigned int function0:3;
    unsigned int function1:16;
    unsigned int function2:16;
    unsigned int function3:16;
  } nan;
  struct 
  {
    unsigned long msw;
    unsigned long lsw;
  } parts;
    long aslong[2];
} __ieee_double_shape_type;

#endif

#ifdef __IEEE_LITTLE_ENDIAN

typedef union 
{
  double value;
  struct 
  {
#ifdef __SMALL_BITFIELDS
    unsigned int fraction3:16;
    unsigned int fraction2:16;
    unsigned int fraction1:16;
    unsigned int fraction0: 4;
#else
    unsigned int fraction1:32;
    unsigned int fraction0:20;
#endif
    unsigned int exponent :11;
    unsigned int sign     : 1;
  } number;
  struct 
  {
#ifdef __SMALL_BITFIELDS
    unsigned int function3:16;
    unsigned int function2:16;
    unsigned int function1:16;
    unsigned int function0:3;
#else
    unsigned int function1:32;
    unsigned int function0:19;
#endif
    unsigned int quiet:1;
    unsigned int exponent: 11;
    unsigned int sign : 1;
  } nan;
  struct 
  {
    unsigned long lsw;
    unsigned long msw;
  } parts;

  long aslong[2];

} __ieee_double_shape_type;

#endif

#ifdef __IEEE_BIG_ENDIAN

typedef union
{
  float value;
  struct 
  {
    unsigned int sign : 1;
    unsigned int exponent: 8;
    unsigned int fraction0: 7;
    unsigned int fraction1: 16;
  } number;
  struct 
  {
    unsigned int sign:1;
    unsigned int exponent:8;
    unsigned int quiet:1;
    unsigned int function0:6;
    unsigned int function1:16;
  } nan;
  long p1;
  
} __ieee_float_shape_type;

#endif

#ifdef __IEEE_LITTLE_ENDIAN

typedef union
{
  float value;
  struct 
  {
    unsigned int fraction0: 7;
    unsigned int fraction1: 16;
    unsigned int exponent: 8;
    unsigned int sign : 1;
  } number;
  struct 
  {
    unsigned int function1:16;
    unsigned int function0:6;
    unsigned int quiet:1;
    unsigned int exponent:8;
    unsigned int sign:1;
  } nan;
  long p1;
  
} __ieee_float_shape_type;

#endif





/* FLOATING ROUNDING */

typedef int fp_rnd;
#define FP_RN 0 	/* Round to nearest 		*/
#define FP_RM 1		/* Round down 			*/
#define FP_RP 2		/* Round up 			*/
#define FP_RZ 3		/* Round to zero (trunate) 	*/

fp_rnd _EXFUN(fpgetround,(void));
fp_rnd _EXFUN(fpsetround, (fp_rnd));

/* EXCEPTIONS */

typedef int fp_except;
#define FP_X_INV 0x10	/* Invalid operation 		*/
#define FP_X_DX  0x80	/* Divide by zero		*/
#define FP_X_OFL 0x04	/* Overflow exception		*/
#define FP_X_UFL 0x02	/* Underflow exception		*/
#define FP_X_IMP 0x01	/* imprecise exception		*/

fp_except _EXFUN(fpgetmask,(void));
fp_except _EXFUN(fpsetmask,(fp_except));
fp_except _EXFUN(fpgetsticky,(void));
fp_except _EXFUN(fpsetsticky, (fp_except));

/* INTEGER ROUNDING */

typedef int fp_rdi;
#define FP_RDI_TOZ 0	/* Round to Zero 		*/
#define FP_RDI_RD  1	/* Follow float mode		*/

fp_rdi _EXFUN(fpgetroundtoi,(void));
fp_rdi _EXFUN(fpsetroundtoi,(fp_rdi));

#undef isnan
#undef isinf

int _EXFUN(isnan, (double));
int _EXFUN(isinf, (double));



int _EXFUN(isnanf, (float));
int _EXFUN(isinff, (float));

#define __IEEE_DBL_EXPBIAS 1023
#define __IEEE_FLT_EXPBIAS 127

#define __IEEE_DBL_EXPLEN 11
#define __IEEE_FLT_EXPLEN 8


#define __IEEE_DBL_FRACLEN (64 - (__IEEE_DBL_EXPLEN + 1))
#define __IEEE_FLT_FRACLEN (32 - (__IEEE_FLT_EXPLEN + 1))

#define __IEEE_DBL_MAXPOWTWO	((double)(1L << 32 - 2) * (1L << (32-11) - 32 + 1))
#define __IEEE_FLT_MAXPOWTWO	((float)(1L << (32-8) - 1))

#define __IEEE_DBL_NAN_EXP 0x7ff
#define __IEEE_FLT_NAN_EXP 0xff

#ifndef __ieeefp_isnanf
#define __ieeefp_isnanf(x) (((*(long *)&(x) & 0x7f800000L)==0x7f800000L) && \
			    ((*(long *)&(x) & 0x007fffffL)!=0000000000L))
#endif
#define isnanf(x)	__ieeefp_isnanf(x)

#ifndef __ieeefp_isinff
#define __ieeefp_isinff(x) (((*(long *)&(x) & 0x7f800000L)==0x7f800000L) && \
			    ((*(long *)&(x) & 0x007fffffL)==0000000000L))
#endif
#define isinff(x)	__ieeefp_isinff(x)

#ifndef __ieeefp_finitef
#define __ieeefp_finitef(x) (((*(long *)&(x) & 0x7f800000L)!=0x7f800000L))
#endif
#define finitef(x)	__ieeefp_finitef(x)

#ifdef _DOUBLE_IS_32BITS
#undef __IEEE_DBL_EXPBIAS
#define __IEEE_DBL_EXPBIAS __IEEE_FLT_EXPBIAS

#undef __IEEE_DBL_EXPLEN
#define __IEEE_DBL_EXPLEN __IEEE_FLT_EXPLEN

#undef __IEEE_DBL_FRACLEN
#define __IEEE_DBL_FRACLEN __IEEE_FLT_FRACLEN

#undef __IEEE_DBL_MAXPOWTWO
#define __IEEE_DBL_MAXPOWTWO __IEEE_FLT_MAXPOWTWO

#undef __IEEE_DBL_NAN_EXP
#define __IEEE_DBL_NAN_EXP __IEEE_FLT_NAN_EXP

#undef __ieee_double_shape_type
#define __ieee_double_shape_type __ieee_float_shape_type

#endif /* _DOUBLE_IS_32BITS */

_END_STD_C

#endif /* _IEEE_FP_H_ */
