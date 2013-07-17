/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/_exit.c - exit() system call.
 */

#include <nanvix/syscall.h>
#include <sys/types.h>
#include <errno.h>

/*
 * Terminates a process.
 */
void _exit(int status)
{
	__asm__ (
		"int $0x80" 
		: /* empty. */
		: "a" (NR_exit), 
		  "b" (status)
	);
}
