/*
 * Copyleft(C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * unistd/ps.c - ps() syscall
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Gets process information
 */
int ps()
{
	ssize_t ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_ps)
	);

	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return ((ssize_t)ret);
}