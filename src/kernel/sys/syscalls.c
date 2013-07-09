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
	(void (*)(void))&sys_alarm,
	(void (*)(void))&sys_brk,
	(void (*)(void))&sys_fork,
	(void (*)(void))&sys_getegid,
	(void (*)(void))&sys_geteuid,
	(void (*)(void))&sys_getgid,
	(void (*)(void))&sys_getpgrp,
	(void (*)(void))&sys_getpid,
	(void (*)(void))&sys_getppid,
	(void (*)(void))&sys_getuid,
	(void (*)(void))&sys_kill,
	(void (*)(void))&sys_nice,
	(void (*)(void))&sys_pause,
	(void (*)(void))&sys_setegid,
	(void (*)(void))&sys_seteuid,
	(void (*)(void))&sys_setgid,
	(void (*)(void))&sys_setpgrp,
	(void (*)(void))&sys_setuid,
	(void (*)(void))&sys_exit,
	(void (*)(void))&sys_wait
};
