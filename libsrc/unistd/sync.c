/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/sync.c - sync() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Schedules file system updates.
 */
void sync(void)
{
	__asm__ volatile(
		"int $0x80"
		: /* empty. */
		: "a" (NR_sync)
	);
}

