/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/_exit.c - _exit() system call.
 */

#include <nanvix/syscall.h>
#include <unistd.h>
#include <errno.h>

/*
 * Terminates the calling process.
 */
void _exit(int status)
{
	__asm__ volatile(
		"int $0x80"
		: /* empty. */
		: "a" (NR__exit),
		"b" (status)
	);
}
