/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef _SAFE_H_
#define _SAFE_H_

	#include <sys/types.h>

	/* Forward definitions. */
	extern int sopen(const char *, int);
	extern void sclose(int);
	extern void slseek(int, off_t, int);
	extern void sread(int, void *, size_t);
	extern void swrite(int, const void *, size_t);
	extern void *smalloc(size_t);
	extern void error(const char *);
	extern const char *break_path(const char *, char *);
	extern void *scalloc(size_t, size_t);

#endif /* _SAFE_H_ */
