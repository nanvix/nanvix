/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <sys/wait.h> - Declarations for waiting.
 */

#ifndef SYS_WAIT_H_
#define SYS_WAIT_H_

	#define WEXITSTATUS(status) \
		(status & 0xff)         \
    
    #define WIFEXITED(status) \
		((status >> 8) & 1)   \
    
    #define WIFSIGNALED(status) \
		((status >> 9) & 1)     \

    #define WTERMSIG(status)    \
		((status >> 16) & 0xff) \
    
    /* Function prototypes. */
    extern pid_t wait(int *stat_loc);

#endif /* WAIT_H_ */
