/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/stat/stat.c - stat() system call.
 */

#include <nanvix/syscall.h>
#include <sys/stat.h>
#include <errno.h>

/*
 * Gets file status.
 */
int stat(const char *path, struct stat *buf)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_stat),
		  "b" (path),
		  "c" (buf)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
