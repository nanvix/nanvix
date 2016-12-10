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

#ifndef _NEWLIB_STDIO_H
#define _NEWLIB_STDIO_H

#include <sys/lock.h>
#include <sys/reent.h>

/* Internal locking macros, used to protect stdio functions.  In the
   general case, expand to nothing. Use __SSTR flag in FILE _flags to
   detect if FILE is private to sprintf/sscanf class of functions; if
   set then do nothing as lock is not initialised. */
#if !defined(_flockfile)
#ifndef __SINGLE_THREAD__
#  define _flockfile(fp) (((fp)->_flags & __SSTR) ? (void)0 : __lock_acquire_recursive((fp)->_lock))
#else
#  define _flockfile(fp)	(_CAST_VOID 0)
#endif
#endif

#if !defined(_funlockfile)
#ifndef __SINGLE_THREAD__
#  define _funlockfile(fp) (((fp)->_flags & __SSTR) ? (void)0 : __lock_release_recursive((fp)->_lock))
#else
#  define _funlockfile(fp)	(_CAST_VOID 0)
#endif
#endif

#endif /* _NEWLIB_STDIO_H */

/*
 * Stdio function-access interface.
 */

/*
 * The __sfoo macros are here so that we can 
 * define function versions in the C library.
 */
#define __sgetc_raw_r(__ptr, __f) (--(__f)->_r < 0 ? __srget_r(__ptr, __f) : (int)(*(__f)->_p++))

/*
 * This has been tuned to generate reasonable code on the vax using pcc
 */
#define  __sputc_raw_r(__ptr, __c, __p) \
	(--(__p)->_w < 0 ? \
	(__p)->_w >= (__p)->_lbfsize ? \
	(*(__p)->_p = (__c)), *(__p)->_p != '\n' ? \
	(int)*(__p)->_p++ : \
	__swbuf_r(__ptr, '\n', __p) : \
	__swbuf_r(__ptr, (int)(__c), __p) : \
	(*(__p)->_p = (__c), (int)*(__p)->_p++))


#define __sgetc_r(__ptr, __p) __sgetc_raw_r(__ptr, __p)
     
#define __sputc_r(__ptr, __c, __p) __sputc_raw_r(__ptr, __c, __p)

#define	__sfeof(p)	((int)(((p)->_flags & __SEOF) != 0))
#define	__sferror(p)	((int)(((p)->_flags & __SERR) != 0))
#define	__sclearerr(p)	((void)((p)->_flags &= ~(__SERR|__SEOF)))
#define	__sfileno(p)	((p)->_file)
