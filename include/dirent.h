/*
 * Copyright(C) 2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <dirent.h> - Format of directory entries.
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

#endif /* DIRENT_H_ */
