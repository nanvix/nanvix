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

#ifndef UTIME_H_
#define UTIME_H_
#ifndef _ASM_FILE_

	#include <sys/types.h>

	/*
	 * Times buffer.
	 */
	struct utimbuf
	{
		time_t actime;  /* Access time.       */
		time_t modtime; /* Modification time. */
	};

	/*
	 * Set file access and modification times
	 */
	extern int utime(const char *path, struct utimbuf *times);

#endif /* _ASM_FILE_ */
#endif /* UTIME_H_ */
