/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <signal/signal.c> - signal() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <signal.h>

void restorer()
{
}

/*
 * Manages signals.
 */
sighandler_t signal(int sig, sighandler_t func)
{
	sighandler_t old_func;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (old_func)
		: "0" (NR_signal),
		  "b" (sig),
		  "c" (func),
		  "d" (restorer)
	);
	
	/* Error. */
	if (old_func == NULL)
	{
		errno = -EINVAL;
		return (SIG_ERR);
	}

	return (old_func);
}
