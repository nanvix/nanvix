/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fcntl/fcntl.c - fcntl() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>

/*
 * Manipulates file descriptor.
 */
int fcntl(int fd, int cmd, ...)
{
	int ret;      /* Return value.      */
	int arg;      /* Creation mode.     */
	va_list varg; /* Variable argument. */
	
	arg = 0;
	
	if (cmd == F_DUPFD)
	{
		va_start(varg, arg);
		arg = va_arg(varg, int)
		va_end(varg);
	}
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_fcntl),
		  "b" (fd),
		  "c" (cmd),
		  "d" (arg)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
