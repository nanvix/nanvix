/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * sys/syscalls.c - System calls.
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
	(void (*)(void))&sys__exit,
	(void (*)(void))&sys_wait,
	(void (*)(void))&sys_signal,
	(void (*)(void))&sys_access,
	(void (*)(void))&sys_chdir,
	(void (*)(void))&sys_chown,
	(void (*)(void))&sys_chroot,
	(void (*)(void))&sys_chmod,
	(void (*)(void))&sys_open,
	(void (*)(void))&sys_umask,
	(void (*)(void))&sys_read,
	(void (*)(void))&sys_write,
	(void (*)(void))&sys_close,
	(void (*)(void))&sys_execve,
	(void (*)(void))&sys_lseek,
	(void (*)(void))&sys_pipe,
	(void (*)(void))&sys_stat,
	(void (*)(void))&sys_fcntl,
	(void (*)(void))&sys_sync,
	(void (*)(void))&sys_unlink,
	(void (*)(void))&sys_dup2,
	(void (*)(void))&sys_ioctl,
	(void (*)(void))&sys_link,
	(void (*)(void))&sys_uname,
	(void (*)(void))&sys_utime,
	(void (*)(void))&sys_ustat,
	(void (*)(void))&sys_times
};
