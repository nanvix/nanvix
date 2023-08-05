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

#ifndef TYPES_H_
#define TYPES_H_

#ifndef _ASM_FILE_

	#include <stdint.h>

	/* Used for system times in clock ticks. */
	typedef int clock_t;

	/* Used for device IDs. */
	typedef unsigned dev_t;

	/* Used for disk addresses. */
	typedef unsigned daddr_t;

	/* Used as a general identifier. */
	typedef int id_t;

	/* Used for some file attributes. */
	typedef int gid_t;

	/* Used for some file attributes. */
	typedef int mode_t;

	/* Used for file sizes. */
	typedef signed off_t;

	/* Used for process IDs and process group IDs. */
	typedef signed pid_t;

	/* Used for time in seconds. */
	typedef signed time_t;

	/* Used for file serial numbers. */
	typedef uint16_t ino_t;

	/* Used for link counts. */
	typedef int nlink_t;

	#define _NEED_SIZE_T
	#include <decl.h>

	/* Used for a count of bytes or an error indication. */
	typedef signed ssize_t;

	/* Used for user IDs. */
	typedef int uid_t;

#endif /* _ASM_FILE */

#endif /* TYPES_H_ */
