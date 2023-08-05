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

/* ctype table definitions for Windows codepage charsets.
   Included by ctype_.c. */

#define _CTYPE_CP437_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_U,	_U, \
	_U,	_L,	_U,	_L,	_L,	_L,	_L,	_L, \
	_L,	_U,	_U,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_U,	_L,	_U,	_L,	_P,	_L, \
	_U,	_U,	_U,	_L,	_P,	_L,	_L,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP437_255 _S|_B
#define _CTYPE_CP720_128_254 \
   	0,	0,	_L,	_L,	0,	_L,	0,	_L, \
	_L,	_L,	_L,	_L,	_L,	0,	0,	0,  \
	0,	_P,	_P,	_L,	_P,	_P,	_L,	_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP720_255 _S|_B
#define _CTYPE_CP737_128_254 \
   	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_P,	_P,	_P,	_U,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP737_255 _S|_B
#define _CTYPE_CP775_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_U,	_L,	_L,	_U,	_U,	_U, \
	_U,	_L,	_U,	_L,	_L,	_U,	_P,	_U, \
	_L,	_U,	_U,	_L,	_P,	_U,	_P,	_P, \
	_U,	_U,	_L,	_U,	_L,	_L,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_U,	_U,	_U, \
	_U,	_P,	_P,	_P,	_P,	_U,	_U,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_U,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U,	_L,	_U,	_U,	_L,	_U,	_P,	_L, \
	_U,	_L,	_U,	_L,	_L,	_U,	_U,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP775_255 _S|_B
#define _CTYPE_CP850_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_U,	_U, \
	_U,	_L,	_U,	_L,	_L,	_L,	_L,	_L, \
	_L,	_U,	_U,	_L,	_P,	_U,	_P,	_L, \
	_L,	_L,	_L,	_L,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_U,	_U,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_L,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_U,	_U,	_U,	_U,	_L,	_U,	_U, \
	_U,	_P,	_P,	_P,	_P,	_P,	_U,	_P, \
	_U,	_L,	_U,	_U,	_L,	_U,	_P,	_L, \
	_U,	_U,	_U,	_U,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP850_255 _S|_B
#define _CTYPE_CP852_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_U,	_L,	_L,	_U,	_U,	_U, \
	_U,	_U,	_L,	_L,	_L,	_U,	_L,	_U, \
	_L,	_U,	_U,	_U,	_L,	_U,	_P,	_L, \
	_L,	_L,	_L,	_L,	_U,	_L,	_U,	_L, \
	_U,	_L,	_P,	_L,	_U,	_L,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_U,	_U,	_U, \
	_U,	_P,	_P,	_P,	_P,	_U,	_L,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_U,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_U,	_U,	_U,	_L,	_U,	_U,	_U, \
	_L,	_P,	_P,	_P,	_P,	_U,	_U,	_P, \
	_U,	_L,	_U,	_U,	_L,	_L,	_U,	_L, \
	_U,	_U,	_L,	_U,	_L,	_U,	_L,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_L,	_U,	_L,	_P
#define _CTYPE_CP852_255 _S|_B
#define _CTYPE_CP855_128_254 \
   	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_U,	_L,	_U,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_L,	_U,	_L, \
	_U,	_P,	_P,	_P,	_P,	_L,	_U,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_L,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_U,	_L,	_U,	_L,	_U,	_L,	_U, \
	_L,	_P,	_P,	_P,	_P,	_U,	_L,	_P, \
	_U,	_L,	_U,	_L,	_U,	_L,	_U,	_L, \
	_U,	_L,	_U,	_L,	_U,	_L,	_U,	_P, \
	_P,	_L,	_U,	_L,	_U,	_L,	_U,	_L, \
	_U,	_L,	_U,	_L,	_U,	_P,	_P
#define _CTYPE_CP855_255 _S|_B
#define _CTYPE_CP857_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_U,	_U, \
	_U,	_L,	_U,	_L,	_L,	_L,	_L,	_L, \
	_U,	_U,	_U,	_L,	_P,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_U,	_U,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_U,	_U,	_U,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_L,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U,	_U,	_U,	0,	_U,	_U, \
	_U,	_P,	_P,	_P,	_P,	_P,	_U,	_P, \
	_U,	_L,	_U,	_U,	_L,	_U,	_P,	0, \
	_P,	_U,	_U,	_U,	_L,	_L,	_P,	_P, \
	_P,	_P,	0,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP857_255 _S|_B
#define _CTYPE_CP858_128_254 \
   	_U,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_U,	_U, \
	_U,	_L,	_U,	_L,	_L,	_L,	_L,	_L, \
	_L,	_U,	_U,	_L,	_P,	_U,	_P,	_L, \
	_L,	_L,	_L,	_L,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_U,	_U,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_L,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_U,	_U,	_U,	_U,	_P,	_U,	_U, \
	_U,	_P,	_P,	_P,	_P,	_P,	_U,	_P, \
	_U,	_L,	_U,	_U,	_L,	_U,	_P,	_L, \
	_U,	_U,	_U,	_U,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP858_255 _S|_B
#define _CTYPE_CP862_128_254 \
   	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_P,	_P,	_P,	_P,	_L, \
	_L,	_L,	_L,	_L,	_L,	_U,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_U,	_L,	_U,	_L,	_P,	_L, \
	_U,	_U,	_U,	_L,	_P,	_L,	_L,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP862_255 _S|_B
#define _CTYPE_CP866_128_254 \
   	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_U,	_L,	_U,	_L,	_U,	_L,	_U,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP866_255 _S|_B
#define _CTYPE_CP874_128_254 \
   	_P,	0,	0,	0,	0,	_P,	0,	0,  \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	0,	0,	0,	0,	0,	0,	0,	0,  \
	_S|_B,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	0,	0,	0,	0,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U|_L,	_U|_L,	0,	0,	0
#define _CTYPE_CP874_255 0
#define _CTYPE_CP1125_128_254 \
   	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_U,	_L,	_U,	_L,	_U,	_L,	_U,	_L, \
	_U,	_L,	_P,	_P,	_P,	_P,	_P
#define _CTYPE_CP1125_255 _S|_B
#define _CTYPE_CP1250_128_254 \
   	_P,	0,	_P,	0,	_P,	_P,	_P,	_P, \
	0,	_P,	_U,	_P,	_U,	_U,	_U,	_U, \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	0,	_P,	_L,	_P,	_L,	_L,	_L,	_L, \
	_S|_B,	_P,	_P,	_U,	_P,	_U,	_P,	_P, \
	_P,	_P,	_U,	_P,	_P,	_P,	_P,	_U, \
	_P,	_P,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_L,	_L,	_P,	_U,	_P,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_CP1250_255 _P
#define _CTYPE_CP1251_128_254 \
   	_U,	_U,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U,	_P,	_U,	_U,	_U,	_U, \
	_L,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	0,	_P,	_L,	_P,	_L,	_L,	_L,	_L, \
	_S|_B,	_U,	_L,	_U,	_P,	_U,	_P,	_P, \
	_U,	_P,	_U,	_P,	_P,	_P,	_P,	_U, \
	_P,	_P,	_U,	_L,	_L,	_P,	_P,	_P, \
	_L,	_P,	_L,	_P,	_L,	_U,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_CP1251_255 _L
#define _CTYPE_CP1252_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U,	_P,	_U,	_U,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_L,	_P,	_L,	0,	_L,	_U, \
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
#define _CTYPE_CP1252_255 _L
#define _CTYPE_CP1253_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	0,	_P,	0,	_P,	0,	0,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	0,	_P,	0,	_P,	0,	0,	0,	0,  \
	_S|_B,	_P,	_U,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	0,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U,	_U,	_U,	_P,	_U,	_P,	_U,	_U, \
	_L,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_CP1253_255 _L
#define _CTYPE_CP1254_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U,	_P,	_U,	0,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_L,	_P,	_L,	0,	0,	_U, \
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
#define _CTYPE_CP1254_255 _L
#define _CTYPE_CP1255_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	0,	_P,	0,	0,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	0,	_P,	0,	0,	0,	0,  \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_U|_L,	_U|_L,	_U|_L,	_P, \
	_P,	0,	0,	0,	0,	0,	0,	0,  \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	0,	0,	_P,	_P
#define _CTYPE_CP1255_255 0
#define _CTYPE_CP1256_128_254 \
   	_P,	_U|_L,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U|_L,	_P,	_U,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U|_L,	_P,	_U|_L,	_P,	_L,	_P,	_P,	_U|_L, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U|_L,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_P,	_U|_L,	_U|_L,	_U|_L, \
	_L,	_U|_L,	_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_L, \
	_L,	_L,	_L,	_L,	_U|_L,	_U|_L,	_L,	_L, \
	_P,	_P,	_P,	_P,	_L,	_P,	_P,	_P, \
	_P,	_L,	_P,	_L,	_L,	_P,	_P
#define _CTYPE_CP1256_255 _U|_L
#define _CTYPE_CP1257_128_254 \
   	_P,	0,	_P,	0,	_P,	_P,	_P,	_P, \
	0,	_P,	0,	_P,	0,	_P,	_P,	_P, \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	0,	_P,	0,	_P,	0,	_P,	_P,	0,  \
	_S|_B,	0,	_P,	_P,	_P,	0,	_P,	_P, \
	_U,	_P,	_U,	_P,	_P,	_P,	_P,	_U, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_P,	_L,	_P,	_P,	_P,	_P,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_CP1257_255 _P
#define _CTYPE_CP1258_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	0,	_P,	_U,	0,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	0,	_P,	_L,	0,	0,	_U, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_P,	_U,	_U,	_U, \
	_U,	_U,	_P,	_U,	_U,	_U,	_U,	_P, \
	_U,	_U,	_U,	_U,	_U,	_U,	_P,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_P,	_L,	_L,	_L, \
	_L,	_L,	_P,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_P
#define _CTYPE_CP1258_255 _L
#define _CTYPE_CP20866_128_254 \
   	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_S|_B,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_U,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U
#define _CTYPE_CP20866_255 _U
#define _CTYPE_CP21866_128_254 \
   	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_S|_B,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_L,	_L,	_P,	_L,	_L, \
	_P,	_P,	_P,	_P,	_P,	_L,	_P,	_P, \
	_P,	_P,	_P,	_U,	_U,	_P,	_U,	_U, \
	_P,	_P,	_P,	_P,	_P,	_U,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U
#define _CTYPE_CP21866_255 _U
#define _CTYPE_GEORGIAN_PS_128_254 \
   	_P,	0,	_P,	_L,	_P,	_P,	_P,	_P, \
	_P,	_P,	_U,	_P,	_U,	_U,	0,	0,  \
	0,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_L,	_P,	_L,	0,	_L,	_U, \
	_S|_B,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_P,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L, \
	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_U|_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_GEORGIAN_PS_255 _L
#define _CTYPE_PT154_128_254 \
   	_U,	_U,	_U,	_L,	_P,	_P,	_U,	_U, \
	_U,	_L,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_P,	_P,	_P,	_P,	_P,	_P,	_P, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_S|_B,	_U,	_L,	_U,	_U,	_U,	_U,	_P, \
	_U,	_P,	_U,	_P,	_P,	_L,	_P,	_U, \
	_P,	_L,	_U,	_L,	_L,	_L,	_P,	_P, \
	_L,	_P,	_L,	_P,	_L,	_U,	_L,	_L, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_U,	_U,	_U,	_U,	_U,	_U,	_U,	_U, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L,	_L, \
	_L,	_L,	_L,	_L,	_L,	_L,	_L
#define _CTYPE_PT154_255 _L

extern int __cp_index (const char *charset_ext);

#if defined(ALLOW_NEGATIVE_CTYPE_INDEX)

#ifndef __CYGWIN__
static _CONST
#endif
char __ctype_cp[26][128 + 256] = {
  { _CTYPE_CP437_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP437_128_254,
    _CTYPE_CP437_255
  },
  { _CTYPE_CP720_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP720_128_254,
    _CTYPE_CP720_255
  },
  { _CTYPE_CP737_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP737_128_254,
    _CTYPE_CP737_255
  },
  { _CTYPE_CP775_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP775_128_254,
    _CTYPE_CP775_255
  },
  { _CTYPE_CP850_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP850_128_254,
    _CTYPE_CP850_255
  },
  { _CTYPE_CP852_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP852_128_254,
    _CTYPE_CP852_255
  },
  { _CTYPE_CP855_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP855_128_254,
    _CTYPE_CP855_255
  },
  { _CTYPE_CP857_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP857_128_254,
    _CTYPE_CP857_255
  },
  { _CTYPE_CP858_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP858_128_254,
    _CTYPE_CP858_255
  },
  { _CTYPE_CP862_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP862_128_254,
    _CTYPE_CP862_255
  },
  { _CTYPE_CP866_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP866_128_254,
    _CTYPE_CP866_255
  },
  { _CTYPE_CP874_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP874_128_254,
    _CTYPE_CP874_255
  },
  { _CTYPE_CP1125_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1125_128_254,
    _CTYPE_CP1125_255
  },
  { _CTYPE_CP1250_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1250_128_254,
    _CTYPE_CP1250_255
  },
  { _CTYPE_CP1251_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1251_128_254,
    _CTYPE_CP1251_255
  },
  { _CTYPE_CP1252_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1252_128_254,
    _CTYPE_CP1252_255
  },
  { _CTYPE_CP1253_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1253_128_254,
    _CTYPE_CP1253_255
  },
  { _CTYPE_CP1254_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1254_128_254,
    _CTYPE_CP1254_255
  },
  { _CTYPE_CP1255_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1255_128_254,
    _CTYPE_CP1255_255
  },
  { _CTYPE_CP1256_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1256_128_254,
    _CTYPE_CP1256_255
  },
  { _CTYPE_CP1257_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1257_128_254,
    _CTYPE_CP1257_255
  },
  { _CTYPE_CP1258_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1258_128_254,
    _CTYPE_CP1258_255
  },
  { _CTYPE_CP20866_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP20866_128_254,
    _CTYPE_CP20866_255
  },
  { _CTYPE_CP21866_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP21866_128_254,
    _CTYPE_CP21866_255
  },
  { _CTYPE_GEORGIAN_PS_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_GEORGIAN_PS_128_254,
    _CTYPE_GEORGIAN_PS_255
  },
  { _CTYPE_PT154_128_254,
    0,
    _CTYPE_DATA_0_127,
    _CTYPE_PT154_128_254,
    _CTYPE_PT154_255
  },
};

#else /* !defined(ALLOW_NEGATIVE_CTYPE_INDEX) */

static _CONST char __ctype_cp[26][1 + 256] = {
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP437_128_254,
    _CTYPE_CP437_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP720_128_254,
    _CTYPE_CP720_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP737_128_254,
    _CTYPE_CP737_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP775_128_254,
    _CTYPE_CP775_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP850_128_254,
    _CTYPE_CP850_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP852_128_254,
    _CTYPE_CP852_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP855_128_254,
    _CTYPE_CP855_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP857_128_254,
    _CTYPE_CP857_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP858_128_254,
    _CTYPE_CP858_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP862_128_254,
    _CTYPE_CP862_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP866_128_254,
    _CTYPE_CP866_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP874_128_254,
    _CTYPE_CP874_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1125_128_254,
    _CTYPE_CP1125_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1250_128_254,
    _CTYPE_CP1250_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1251_128_254,
    _CTYPE_CP1251_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1252_128_254,
    _CTYPE_CP1252_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1253_128_254,
    _CTYPE_CP1253_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1254_128_254,
    _CTYPE_CP1254_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1255_128_254,
    _CTYPE_CP1255_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1256_128_254,
    _CTYPE_CP1256_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1257_128_254,
    _CTYPE_CP1257_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP1258_128_254,
    _CTYPE_CP1258_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP20866_128_254,
    _CTYPE_CP20866_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_CP21866_128_254,
    _CTYPE_CP21866_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_GEORGIAN_PS_128_254,
    _CTYPE_GEORGIAN_PS_255
  },
  { 0,
    _CTYPE_DATA_0_127,
    _CTYPE_PT154_128_254,
    _CTYPE_PT154_255
  },
};

#endif /* ALLOW_NEGATIVE_CTYPE_INDEX */
