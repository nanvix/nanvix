/*
 * Copyright (C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/stat.h> - File status library header.
 */

#ifndef STAT_H_
#define STAT_H_
	
	#include <sys/types.h>
	
	/* File types. */
	#define S_IFMT  00170000
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
	
	/*
	 * Changes permissions of a file.
	 */
	extern int chmod(const char *path, mode_t mode);
	
	/*
	 * Sets and gets the file mode creation mask.
	 */
	extern mode_t umask(mode_t cmask);

#endif /* STAT_H_ */
