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

#ifdef __IEEE_BIG_ENDIAN

#if !defined(__mips)
#define f_QNAN 0x7fc00000
#define d_QNAN0 0x7ff80000
#define d_QNAN1 0x0
#define ld_QNAN0 0x7ff80000
#define ld_QNAN1 0x0
#define ld_QNAN2 0x0
#define ld_QNAN3 0x0
#define ldus_QNAN0 0x7ff8
#define ldus_QNAN1 0x0
#define ldus_QNAN2 0x0
#define ldus_QNAN3 0x0
#define ldus_QNAN4 0x0
#elif defined(__mips_nan2008)
#define f_QNAN 0x7fc00000
#define d_QNAN0 0x7ff80000
#define d_QNAN1 0x0
#else
#define f_QNAN 0x7fbfffff
#define d_QNAN0 0x7ff7ffff
#define d_QNAN1 0xffffffff
#endif

#elif defined(__IEEE_LITTLE_ENDIAN)

#if !defined(__mips)
#define f_QNAN 0xffc00000
#define d_QNAN0 0x0
#define d_QNAN1 0xfff80000
#define ld_QNAN0 0x0
#define ld_QNAN1 0xc0000000
#define ld_QNAN2 0xffff
#define ld_QNAN3 0x0
#define ldus_QNAN0 0x0
#define ldus_QNAN1 0x0
#define ldus_QNAN2 0x0
#define ldus_QNAN3 0xc000
#define ldus_QNAN4 0xffff
#elif defined(__mips_nan2008)
#define f_QNAN 0x7fc00000
#define d_QNAN0 0x0
#define d_QNAN1 0x7ff80000
#else
#define f_QNAN 0x7fbfffff
#define d_QNAN0 0xffffffff
#define d_QNAN1 0x7ff7ffff
#endif

#else
#error IEEE endian not defined
#endif
