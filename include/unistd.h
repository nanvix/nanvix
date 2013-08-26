/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd.h> - Standard symbolic constants and types.
 */

#ifndef UNISTD_H_
#define UNISTD_H_

	#include <sys/types.h>

	extern unsigned alarm(unsigned seconds);
	extern void _exit(int status);
	extern pid_t getpid(void);
	extern pid_t getpgrp(void);
	extern int pause(void);

	/*
	 * Gets the effective user group ID of the calling process.
	 */
	extern gid_t getegid(void);
	
	/*
	 *  Gets the effective user ID of the calling process.
	 */
	extern uid_t geteuid(void);
	
	/*
	 * Gets the real user group ID of the calling process.
	 */
	extern gid_t getgid(void);
	
	/*
	 * Gets the parent process ID of the calling process.
	 */
	extern pid_t getppid(void);
	
	/*
	 * Gets the real user ID of the calling process.
	 */
	extern uid_t sys_getuid(void);
	
	/*
	 * Sets the effective user group ID of the calling process.
	 */
	extern int setegid(gid_t gid);
	
	/*
	 * Sets the user group ID of the calling process.
	 */
	extern int setgid(gid_t gid);

#endif /* UNISTD_H_ */
