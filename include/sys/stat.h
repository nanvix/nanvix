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

#ifndef STAT_H_
#define STAT_H_

	#include <sys/types.h>

	/* File types. */
    #ifndef __APPLE__
	    #define S_IFMT  00170000
    #endif /* __APPLE__ */
	#define S_IFREG  0100000
	#define S_IFBLK  0060000
	#define S_IFDIR  0040000
	#define S_IFCHR  0020000
	#define S_IFIFO  0010000

	#define S_ISREG(m)	(((m) & S_IFMT) == S_IFREG) /* Regular file?       */
	#define S_ISDIR(m)	(((m) & S_IFMT) == S_IFDIR) /* Directory?          */
	#define S_ISCHR(m)	(((m) & S_IFMT) == S_IFCHR) /* Char. special file? */
	#define S_ISBLK(m)	(((m) & S_IFMT) == S_IFBLK) /* Block special file? */
	#define S_ISFIFO(m)	(((m) & S_IFMT) == S_IFIFO) /* FIFO special file?  */

	/* Mode bits. */
    #ifndef __APPLE__
        #define S_IRWXU  0700 /* Read, write, execute/search by owner.     */
        #define S_IRUSR  0400 /* Read permission, owner.                   */
        #define S_IWUSR  0200 /* Write permission, owner.                  */
        #define S_IXUSR  0100 /* Execute/search permission, owner.         */
        #define S_IRWXG   070 /* Read, write, execute/search by group.     */
        #define S_IRGRP   040 /* Read permission, group.                   */
        #define S_IWGRP   020 /* Write permission, group.                  */
        #define S_IXGRP   010 /* Execute/search permission, group.         */
        #define S_IRWXO    07 /* Read, write, execute/search by others.    */
        #define S_IROTH    04 /* Read permission, others.                  */
        #define S_IWOTH    02 /* Write permission, others.                 */
        #define S_IXOTH    01 /* Execute/search permission, others.        */
        #define S_ISUID 04000 /* Set-user-ID on execution.                 */
        #define S_ISGID 02000 /* Set-group-ID on execution.                */
        #define S_ISVTX 01000 /* On directories, restricted deletion flag. */
    #endif /* __APPLE__ */

#ifndef _ASM_FILE_

	/*
	 * File status.
	 */
	struct stat
	{
		dev_t st_dev;     /* Device ID of the containing file.              */
		ino_t st_ino;     /* File serial number.                            */
		mode_t st_mode;   /* File mode (see above).                         */
		nlink_t st_nlink; /* Number of hard links to the file.              */
		uid_t st_uid;     /* User ID of file.                               */
		gid_t st_gid;     /* Group ID of file.                              */
		off_t st_size;    /* For regular files, the file size in bytes.
                           * For symbolic links, the length in bytes of the
                           * pathname contained in the symbolic link.       */
		time_t st_atime;  /* Time of last access.                           */
		time_t st_mtime;  /* Time of last data modification.                */
		time_t st_ctime;  /* Time of last status change.                    */
	};

	/*
	 * Changes permissions of a file.
	 */
	extern int chmod(const char *path, mode_t mode);

	/*
	 * Gets file status.
	 */
	extern int stat(const char *path, struct stat *buf);

	/*
	 * Sets and gets the file mode creation mask.
	 */
	extern mode_t umask(mode_t cmask);

#endif /* _ASM_FILE_ */

#endif /* STAT_H_ */
