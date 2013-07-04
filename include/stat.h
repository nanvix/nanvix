/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stat.h - File status library header.
 */

#ifndef STAT_H_
#define STAT_H_

	/* Mode bits. */
	#define S_IRWXU 0700  /* Read, write, execute/search by owner.     */
	#define S_IRUSR 0400  /* Read permission, owner.                   */
	#define S_IWUSR 0200  /* Write permission, owner.                  */
	#define S_IXUSR 0100  /* Execute/search permission, owner.         */
	#define S_IRWXG 070   /* Read, write, execute/search by group.     */
	#define S_IRGRP 040   /* Read permission, group.                   */
	#define S_IWGRP 020   /* Write permission, group.                  */
	#define S_IXGRP 010   /* Execute/search permission, group.         */
	#define S_IRWXO 07    /* Read, write, execute/search by others.    */
	#define S_IROTH 04    /* Read permission, others.                  */
	#define S_IWOTH 02    /* Write permission, others.                 */
	#define S_IXOTH 01    /* Execute/search permission, others.        */
	#define S_ISUID 04000 /* Set-user-ID on execution.                 */
	#define S_ISGID 02000 /* Set-group-ID on execution.                */
	#define S_ISVTX 01000 /* On directories, restricted deletion flag. */

#endif /* STAT_H_ */
