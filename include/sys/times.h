/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/times.h> - Process times structure.
 */

#ifndef TIMES_H_
#define TIMES_H_
#ifndef _ASM_FILE_

	#include <sys/times.h>

	/*
	 * Process times.
	 */
	struct tms
	{
		clock_t tms_utime;  /* User CPU time.                                 */
		clock_t tms_stime;  /* System CPU time.                               */
		clock_t tms_cutime; /* User CPU time of terminated child processes.   */
		clock_t tms_cstime; /* System CPU time of terminated child processes. */
	};
	
	/*
	 * Gets process and waited-for child process times.
	 */
	extern clock_t times(struct tms *buffer);

#endif /* _ASM_FILE_ */
#endif /* TIMES_H_ */
