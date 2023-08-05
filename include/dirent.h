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

#ifndef DIRENT_H_
#define DIRENT_H_

	#include <sys/types.h>
	#include <limits.h>

	/*
	 * Directory entry.
	 */
	struct dirent
	{
		ino_t d_ino;           /* File serial number. */
		char d_name[NAME_MAX]; /* Name of entry.      */
	};

	/* Directory stream buffer size. */
	#define _DIR_BUFSIZ ((1024/sizeof(struct dirent))*sizeof(struct dirent))

	/* Directory stream flags. */
	#define _DIR_VALID 001 /* Valid directory?  */
	#define _DIR_EOD   002 /* End of directory? */

	/*
	 * Directory stream.
	 */
	typedef struct
	{
		int fd;             /* Underlying file descriptor.       */
		int flags;          /* Flags (see above).                */
		int count;          /* Valid entries left in the buffer. */
		struct dirent *ptr; /* Next valid entry in the buffer.   */
		struct dirent *buf; /* Buffer of directory entries.      */
	} DIR;

	/*
	 * Closes a directory stream.
	 */
	extern int closedir(DIR *dirp);

	/*
	 * Opens a directory stream.
	 */
	extern DIR *opendir(const char *dirname);

	/*
	 * Reads a directory.
	 */
	extern struct dirent *readdir(DIR *dirp);

	/*
	 * Rewinds a directory stream.
	 */
	extern void rewinddir(DIR *dirp);

#endif /* DIRENT_H_ */
