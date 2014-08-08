/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * dirent/dirent.h - Private dirent library.
 */

#ifndef _DIRENT_H_
#define _DIRENT_H_

	#include <dirent.h>
	#include <limits.h>

	/* Directory stream table. */
	extern DIR dirs[OPEN_MAX];

#endif /* _DIRENT_H_ */
