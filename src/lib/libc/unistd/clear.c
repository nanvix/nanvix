/*
 * Copyleft(C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * unistd/clear.c - clear() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Clear the screen
 */
int clear()
{
	ssize_t ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_clear)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return ((ssize_t)ret);
}