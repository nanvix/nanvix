/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * die.c - Exceptions.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>

/*
 * 
 */
PUBLIC void die()
{
	while (1)
		yield();
}

/*
 * Terminates a process.
 */
PUBLIC void terminate(int err)
{
	kprintf("terminating process %d with error code %d", curr_proc->pid, err);
	
	die();
}

/*
 * Aborts the execution of a process.
 */
PUBLIC void abort()
{
	kprintf("aborting process %d", curr_proc->pid);
	
	die();
}
