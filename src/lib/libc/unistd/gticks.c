/*
 * Copyleft(C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * unistd/gticks.c - gticks() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Gets sys ticks since initialization, may be useful
 * as seed for random numbers, but mktime would be better.
 */
int gticks()
{
	ssize_t ret = 0;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_gticks)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return ((ssize_t)ret);
}