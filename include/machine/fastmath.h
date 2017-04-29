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

#ifdef __sysvnecv70_target
double EXFUN(fast_sin,(double));
double EXFUN(fast_cos,(double));
double EXFUN(fast_tan,(double));

double EXFUN(fast_asin,(double));
double EXFUN(fast_acos,(double));
double EXFUN(fast_atan,(double));

double EXFUN(fast_sinh,(double));
double EXFUN(fast_cosh,(double));
double EXFUN(fast_tanh,(double));

double EXFUN(fast_asinh,(double));
double EXFUN(fast_acosh,(double));
double EXFUN(fast_atanh,(double));

double EXFUN(fast_abs,(double));
double EXFUN(fast_sqrt,(double));
double EXFUN(fast_exp2,(double));
double EXFUN(fast_exp10,(double));
double EXFUN(fast_expe,(double));
double EXFUN(fast_log10,(double));
double EXFUN(fast_log2,(double));
double EXFUN(fast_loge,(double));


#define	sin(x)		fast_sin(x)
#define	cos(x)		fast_cos(x)
#define	tan(x)		fast_tan(x)
#define	asin(x)		fast_asin(x)
#define	acos(x)		fast_acos(x)
#define	atan(x)		fast_atan(x)
#define	sinh(x)		fast_sinh(x)
#define	cosh(x)		fast_cosh(x)
#define	tanh(x)		fast_tanh(x)
#define	asinh(x)	fast_asinh(x)
#define	acosh(x)	fast_acosh(x)
#define	atanh(x)	fast_atanh(x)
#define	abs(x)		fast_abs(x)
#define	sqrt(x)		fast_sqrt(x)
#define	exp2(x)		fast_exp2(x)
#define	exp10(x)	fast_exp10(x)
#define	expe(x)		fast_expe(x)
#define	log10(x)	fast_log10(x)
#define	log2(x)		fast_log2(x)
#define	loge(x)		fast_loge(x)

#ifdef _HAVE_STDC
/* These functions are in assembler, they really do take floats. This
   can only be used with a real ANSI compiler */

float EXFUN(fast_sinf,(float));
float EXFUN(fast_cosf,(float));
float EXFUN(fast_tanf,(float));

float EXFUN(fast_asinf,(float));
float EXFUN(fast_acosf,(float));
float EXFUN(fast_atanf,(float));

float EXFUN(fast_sinhf,(float));
float EXFUN(fast_coshf,(float));
float EXFUN(fast_tanhf,(float));

float EXFUN(fast_asinhf,(float));
float EXFUN(fast_acoshf,(float));
float EXFUN(fast_atanhf,(float));

float EXFUN(fast_absf,(float));
float EXFUN(fast_sqrtf,(float));
float EXFUN(fast_exp2f,(float));
float EXFUN(fast_exp10f,(float));
float EXFUN(fast_expef,(float));
float EXFUN(fast_log10f,(float));
float EXFUN(fast_log2f,(float));
float EXFUN(fast_logef,(float));
#define	sinf(x)		fast_sinf(x)
#define	cosf(x)		fast_cosf(x)
#define	tanf(x)		fast_tanf(x)
#define	asinf(x)	fast_asinf(x)
#define	acosf(x)	fast_acosf(x)
#define	atanf(x)	fast_atanf(x)
#define	sinhf(x)	fast_sinhf(x)
#define	coshf(x)	fast_coshf(x)
#define	tanhf(x)	fast_tanhf(x)
#define	asinhf(x)	fast_asinhf(x)
#define	acoshf(x)	fast_acoshf(x)
#define	atanhf(x)	fast_atanhf(x)
#define	absf(x)		fast_absf(x)
#define	sqrtf(x)	fast_sqrtf(x)
#define	exp2f(x)	fast_exp2f(x)
#define	exp10f(x)	fast_exp10f(x)
#define	expef(x)	fast_expef(x)
#define	log10f(x)	fast_log10f(x)
#define	log2f(x)	fast_log2f(x)
#define	logef(x)	fast_logef(x)
#endif
/* Override the functions defined in math.h */
#endif /* __sysvnecv70_target */

