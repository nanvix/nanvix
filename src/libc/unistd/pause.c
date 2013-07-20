/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <unistd/pause.c> - pause() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Suspends the calling process until a signal is received.
 */
int pause(void)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_pause)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
