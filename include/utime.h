/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <utime.h> - Access and modification times structure.
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
