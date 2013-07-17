/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * exit.c - exit() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/pm.h>

/*
 * Terminates the calling process.
 */
PUBLIC void sys_exit(int status)
{
	/* Process exited normally. */
	curr_proc->status = (1 << 8) | (status & 0xff);
	
	die();
}
