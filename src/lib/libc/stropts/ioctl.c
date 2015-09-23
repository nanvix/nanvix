/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stropts/ioctl.c - ioctl() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <stdarg.h>
#include <stropts.h>
#include <unistd.h>

/*
 * Performs control operations on a device.
 */
int ioctl(int fd, int cmd, ...)
{
	int ret;       /* Return value.      */
	unsigned carg; /* Command argument.  */
	va_list arg;   /* Variable argument. */
	
	carg = 0;
	if (IOCTL_MAJOR(cmd) & 1)
	{
		va_start(arg, cmd);
		carg = va_arg(arg, unsigned)
		va_end(arg);
	}
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_ioctl),
		  "b" (fd),
		  "c" (cmd),
		  "d" (carg)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
