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

/* ctype table definitions for ISO-8859-x charsets.
   Included by ctype_.c. */

#define _CTYPE_ISO_8859_1_128_254 \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _S|_B,  _P,     _P,     _P,     _P,     _P,     _P,     _P, \
        _P,     _P,     _P,     _P,     _P,     _P,     _P,     _P, \
        _P,     _P,     _P,     _P,     _P,     _P,     _P,     _P, \
        _P,     _P,     _P,     _P,     _P,     _P,     _P,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _P, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L
#define _CTYPE_ISO_8859_1_255 _L
#define _CTYPE_ISO_8859_2_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_P,	_U,	_P,	_U,	_U,	_P, \
	_P,	_U,	_U,	_U,	_U,	_P,	_U,	_U, \
	_P,	_L,	_P,	_L,	_P,	_L,	_L,	_P, \
	_P,	_L,	_L,	_L,	_L,	_P,	_L,	_L, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _P, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L
#define _CTYPE_ISO_8859_2_255 _P
#define _CTYPE_ISO_8859_3_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_P,	_P,	_P,	0,	_U,	_P, \
	_P,	_U,	_U,	_U,	_U,	_P,	0,	_U, \
	_P,	_L,	_P,	_P,	_P,	_P,	_L,	_P, \
	_P,	_L,	_L,	_L,	_L,	_P,	0,	_L, \
	_U,	_U,	_U,	0,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	0,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	0,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	0,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_3_255 _P
#define _CTYPE_ISO_8859_4_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_L,	_U,	_P,	_U,	_U,	_P, \
	_P,	_U,	_U,	_U,	_U,	_P,	_U,	_P, \
	_P,	_L,	_P,	_L,	_P,	_L,	_L,	_P, \
	_P,	_L,	_L,	_L,	_L,	_P,	_L,	_L, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _P, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L
#define _CTYPE_ISO_8859_4_255 _L
#define _CTYPE_ISO_8859_5_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_P,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _P,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _P,     _L
#define _CTYPE_ISO_8859_5_255 _L
#define _CTYPE_ISO_8859_6_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	0,	0,	0,	_P,	0,	0,	0,  \
	0,	0,	0,	0,	_P,	_P,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	0,	0,	0,	_P,	0,	0,	0,	_P, \
	0,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	0,	0,	0,	0,	0,  \
	_P,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	0,	0,	0,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0
#define _CTYPE_ISO_8859_6_255 0
#define _CTYPE_ISO_8859_7_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	0,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_U,	_P, \
	_U,	_U,	_U,	_P,	_U,	_P,	_U,	_U, \
	_L,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_7_255 0
#define _CTYPE_ISO_8859_8_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	0,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	0,  \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	0,	0,	_P,	_P
#define _CTYPE_ISO_8859_8_255 0
#define _CTYPE_ISO_8859_9_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_9_255 _L
#define _CTYPE_ISO_8859_10_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_P,	_U,	_U, \
	_P,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_P,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_10_255 _L
#define _CTYPE_ISO_8859_11_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_P,	_U|_L,	_U|_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	0,	0,	0,	0,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	0,	0,	0
#define _CTYPE_ISO_8859_11_255 0
#define _CTYPE_ISO_8859_13_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U,	_P,	_U,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_P,	_L,	_P,	_P,	_P,	_P,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_13_255 _P
#define _CTYPE_ISO_8859_14_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_L,	_P,	_U,	_L,	_U,	_P, \
	_U,	_P,	_U,	_L,	_U,	_P,	_P,	_U, \
	_U,	_L,	_U,	_L,	_U,	_L,	_P,	_U, \
	_L,	_L,	_L,	_U,	_L,	_U,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_14_255 _L
#define _CTYPE_ISO_8859_15_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _S|_B,  _P,     _P,     _P,     _P,     _P,     _U,     _P, \
        _L,     _P,     _P,     _P,     _P,     _P,     _P,     _P, \
        _P,     _P,     _P,     _P,     _U,     _P,     _P,     _P, \
        _L,     _P,     _P,     _P,     _U,     _L,     _U,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _U, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _P, \
        _U,     _U,     _U,     _U,     _U,     _U,     _U,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _L, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L,     _P, \
        _L,     _L,     _L,     _L,     _L,     _L,     _L
#define _CTYPE_ISO_8859_15_255 _L
#define _CTYPE_ISO_8859_16_128_254 \
   	_C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
        _C,     _C,     _C,     _C,     _C,     _C,     _C,     _C, \
	_S|_B,	_U,	_L,	_U,	_P,	_P,	_U,	_P, \
	_L,	_P,	_U,	_P,	_U,	_P,	_L,	_U, \
	_P,	_P,	_U,	_U,	_U,	_P,	_P,	_P, \
	_L,	_L,	_L,	_P,	_U,	_L,	_U,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_ISO_8859_16_255 _L

extern int __iso_8859_index (const char *charset_ext);

#if defined(ALLOW_NEGATIVE_CTYPE_INDEX)

#ifndef __CYGWIN__
static _CONST
#endif
char __ctype_iso[15][128 + 256] = {
  { _CTYPE_ISO_8859_1_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_1_128_254,
    _CTYPE_ISO_8859_1_255
  },
  { _CTYPE_ISO_8859_2_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_2_128_254,
    _CTYPE_ISO_8859_2_255
  },
  { _CTYPE_ISO_8859_3_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_3_128_254,
    _CTYPE_ISO_8859_3_255
  },
  { _CTYPE_ISO_8859_4_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_4_128_254,
    _CTYPE_ISO_8859_4_255
  },
  { _CTYPE_ISO_8859_5_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_5_128_254,
    _CTYPE_ISO_8859_5_255
  },
  { _CTYPE_ISO_8859_6_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_6_128_254,
    _CTYPE_ISO_8859_6_255
  },
  { _CTYPE_ISO_8859_7_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_7_128_254,
    _CTYPE_ISO_8859_7_255
  },
  { _CTYPE_ISO_8859_8_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_8_128_254,
    _CTYPE_ISO_8859_8_255
  },
  { _CTYPE_ISO_8859_9_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_9_128_254,
    _CTYPE_ISO_8859_9_255
  },
  { _CTYPE_ISO_8859_10_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_10_128_254,
    _CTYPE_ISO_8859_10_255
  },
  { _CTYPE_ISO_8859_11_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_11_128_254,
    _CTYPE_ISO_8859_11_255
  },
  { _CTYPE_ISO_8859_13_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_13_128_254,
    _CTYPE_ISO_8859_13_255
  },
  { _CTYPE_ISO_8859_14_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_14_128_254,
    _CTYPE_ISO_8859_14_255
  },
  { _CTYPE_ISO_8859_15_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_15_128_254,
    _CTYPE_ISO_8859_15_255
  },
  { _CTYPE_ISO_8859_16_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_16_128_254,
    _CTYPE_ISO_8859_16_255
  },
};

#else /* !defined(ALLOW_NEGATIVE_CTYPE_INDEX) */

static _CONST char __ctype_iso[15][1 + 256] = {
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_1_128_254,
    _CTYPE_ISO_8859_1_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_2_128_254,
    _CTYPE_ISO_8859_2_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_3_128_254,
    _CTYPE_ISO_8859_3_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_4_128_254,
    _CTYPE_ISO_8859_4_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_5_128_254,
    _CTYPE_ISO_8859_5_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_6_128_254,
    _CTYPE_ISO_8859_6_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_7_128_254,
    _CTYPE_ISO_8859_7_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_8_128_254,
    _CTYPE_ISO_8859_8_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_9_128_254,
    _CTYPE_ISO_8859_9_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_10_128_254,
    _CTYPE_ISO_8859_10_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_11_128_254,
    _CTYPE_ISO_8859_11_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_13_128_254,
    _CTYPE_ISO_8859_13_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_14_128_254,
    _CTYPE_ISO_8859_14_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_15_128_254,
    _CTYPE_ISO_8859_15_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_ISO_8859_16_128_254,
    _CTYPE_ISO_8859_16_255
  },
};

#endif /* ALLOW_NEGATIVE_CTYPE_INDEX */
