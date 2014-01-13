/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/close.c - close() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Closes a file.
 */
int close(int fd)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_close),
		  "b" (fd)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
