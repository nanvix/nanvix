/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fcntl/open.c - open() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>

/*
 * Opens a file.
 */
int open(const char *path, int oflag, ...)
{
	int ret;     /* Return value.      */
	mode_t mode; /* Creation mode.     */
	va_list arg; /* Variable argument. */
	
	mode = 0;
	
	if (oflag & O_CREAT)
	{
		va_start(arg, oflag);
		mode = va_arg(arg, mode_t)
		va_end(arg);
	}
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_open),
		  "b" (path),
		  "c" (oflag),
		  "d" (mode)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
