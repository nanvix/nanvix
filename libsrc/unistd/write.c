/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/write.c - write() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Writes to a file.
 */
ssize_t write(int fd, const void *buf, size_t n)
{
	ssize_t ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_write),
		  "b" (fd),
		  "c" (buf),
		  "d" (n)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return ((ssize_t)ret);
}
