/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/chdir.c - chdir() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Closes a file.
 */
int chdir(const char *path)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_chdir),
		  "b" (path)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
