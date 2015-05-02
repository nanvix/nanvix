/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/wait.h> - Declarations for waiting.
 */

#ifndef SYS_WAIT_H_
#define SYS_WAIT_H_

	#include <sys/types.h>
	
	#define _NEED_WSTATUS
	#include <decl.h>
    
    /*
	 * Waits for a child process to stop or terminate.
	 */
	extern pid_t wait(int *stat_loc);

#endif /* WAIT_H_ */
