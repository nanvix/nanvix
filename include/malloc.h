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

/* malloc.h -- header file for memory routines.  */

#ifndef _INCLUDE_MALLOC_H_
#define _INCLUDE_MALLOC_H_

#include <_ansi.h>
#include <sys/reent.h>

#define __need_size_t
#include <stddef.h>

/* include any machine-specific extensions */
#include <machine/malloc.h>

#ifdef __cplusplus
extern "C" {
#endif

/* This version of struct mallinfo must match the one in
   libc/stdlib/mallocr.c.  */

struct mallinfo {
  size_t arena;    /* total space allocated from system */
  size_t ordblks;  /* number of non-inuse chunks */
  size_t smblks;   /* unused -- always zero */
  size_t hblks;    /* number of mmapped regions */
  size_t hblkhd;   /* total space in mmapped regions */
  size_t usmblks;  /* unused -- always zero */
  size_t fsmblks;  /* unused -- always zero */
  size_t uordblks; /* total allocated space */
  size_t fordblks; /* total non-inuse space */
  size_t keepcost; /* top-most, releasable (via malloc_trim) space */
};	

/* The routines.  */

#ifdef __CYGWIN__
#undef _malloc_r
#define _malloc_r(r, s) malloc (s)
#else
#endif

#ifdef __CYGWIN__
#undef _free_r
#define _free_r(r, p) free (p)
#else
#endif

#ifdef __CYGWIN__
#undef _realloc_r
#define _realloc_r(r, p, s) realloc (p, s)
#else
#endif

#ifdef __CYGWIN__
#undef _calloc_r
#define _calloc_r(r, s1, s2) calloc (s1, s2);
#else
#endif

extern _PTR memalign _PARAMS ((size_t, size_t));
#ifdef __CYGWIN__
#undef _memalign_r
#define _memalign_r(r, s1, s2) memalign (s1, s2);
#else
extern _PTR _memalign_r _PARAMS ((struct _reent *, size_t, size_t));
#endif

extern struct mallinfo mallinfo _PARAMS ((void));
#ifdef __CYGWIN__
#undef _mallinfo_r
#define _mallinfo_r(r) mallinfo ()
#else
extern struct mallinfo _mallinfo_r _PARAMS ((struct _reent *));
#endif

extern void malloc_stats _PARAMS ((void));
#ifdef __CYGWIN__
#undef _malloc_stats_r
#define _malloc_stats_r(r) malloc_stats ()
#else
extern void _malloc_stats_r _PARAMS ((struct _reent *));
#endif

extern int mallopt _PARAMS ((int, int));
#ifdef __CYGWIN__
#undef _mallopt_r
#define _mallopt_r(i1, i2) mallopt (i1, i2)
#else
extern int _mallopt_r _PARAMS ((struct _reent *, int, int));
#endif

extern size_t malloc_usable_size _PARAMS ((_PTR));
#ifdef __CYGWIN__
#undef _malloc_usable_size_r
#define _malloc_usable_size_r(r, p) malloc_usable_size (p)
#else
extern size_t _malloc_usable_size_r _PARAMS ((struct _reent *, _PTR));
#endif

/* These aren't too useful on an embedded system, but we define them
   anyhow.  */

extern _PTR valloc _PARAMS ((size_t));
#ifdef __CYGWIN__
#undef _valloc_r
#define _valloc_r(r, s) valloc (s)
#else
extern _PTR _valloc_r _PARAMS ((struct _reent *, size_t));
#endif

extern _PTR pvalloc _PARAMS ((size_t));
#ifdef __CYGWIN__
#undef _pvalloc_r
#define _pvalloc_r(r, s) pvalloc (s)
#else
extern _PTR _pvalloc_r _PARAMS ((struct _reent *, size_t));
#endif

extern int malloc_trim _PARAMS ((size_t));
#ifdef __CYGWIN__
#undef _malloc_trim_r
#define _malloc_trim_r(r, s) malloc_trim (s)
#else
extern int _malloc_trim_r _PARAMS ((struct _reent *, size_t));
#endif

/* A compatibility routine for an earlier version of the allocator.  */

extern _VOID mstats _PARAMS ((char *));
#ifdef __CYGWIN__
#undef _mstats_r
#define _mstats_r(r, p) mstats (p)
#else
#endif

/* SVID2/XPG mallopt options */

#define M_MXFAST  1    /* UNUSED in this malloc */
#define M_NLBLKS  2    /* UNUSED in this malloc */
#define M_GRAIN   3    /* UNUSED in this malloc */
#define M_KEEP    4    /* UNUSED in this malloc */

/* mallopt options that actually do something */
  
#define M_TRIM_THRESHOLD    -1
#define M_TOP_PAD           -2
#define M_MMAP_THRESHOLD    -3 
#define M_MMAP_MAX          -4

#ifndef __CYGWIN__
/* Some systems provide this, so do too for compatibility.  */
#endif /* __CYGWIN__ */

#ifdef __cplusplus
}
#endif

#endif /* _INCLUDE_MALLOC_H_ */
