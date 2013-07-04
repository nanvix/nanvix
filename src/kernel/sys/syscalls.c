/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * syscalls.c - System calls
 */

#include <nanvix/const.h>
#include <nanvix/syscall.h>

/*
 * System calls table.
 */
PUBLIC void (*syscalls_table[NR_SYSCALLS])(void)  = {
	(void (*)(void))&sys_fork
};
