/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd/setegid.c> - setegid() system call.
 */

#include <nanvix/syscall.h>
#include <sys/types.h>
#include <errno.h> 

/*
 * Sets the effective user group ID of the calling process.
 */
int setegid(gid_t gid)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_setegid),
		  "b" (gid)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
