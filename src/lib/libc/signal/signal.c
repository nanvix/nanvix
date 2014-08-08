/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal/signal.c - signal() system call.
 */

#include <nanvix/syscall.h>
#include <signal.h>
#include <errno.h>

/*
 * Manages signal handling.
 */
sighandler_t signal(int sig, sighandler_t func)
{
	sighandler_t ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_signal),
		  "b" (sig),
		  "c" (func)
	);
	
	/* Error. */
	if (ret == SIG_ERR)
	{
		errno = EINVAL;
		return (SIG_ERR);
	}
	
	return (ret);
}
