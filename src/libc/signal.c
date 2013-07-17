/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * signal/signal.c - signal() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>
#include <signal.h>

/*
 * Configures signal handling.
 */
sighandler_t signal(int signum, sighandler_t handler, void (*restorer)(void))
{
	sighandler_t old_handler;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (old_handler)
		: "0" (NR_signal),
		  "b" (signum),
		  "c" (handler),
		  "d" (restorer)
	);
	
	/* Error. */
	if (old_handler == NULL)
	{
		errno = -EINVAL;
		return (NULL);
	}

	return (old_handler);
}
