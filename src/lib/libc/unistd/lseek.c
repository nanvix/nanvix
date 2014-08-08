/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/lseek.c - lseek() system call.
 */

#include <nanvix/syscall.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Moves the read/write file offset.
 */
off_t lseek(int fd, off_t offset, int whence)
{
	__asm__ volatile (
		"int $0x80"
		: "=a" (offset)
		: "0" (NR_lseek),
		  "b" (fd),
		  "c" (offset),
		  "d" (whence)
	);
	
	/* Error. */
	if (offset < 0)
	{
		errno = -offset;
		return (-1);
	}
	
	return (offset);
}

