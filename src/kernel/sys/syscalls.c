/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
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
	(void (*)(void))&sys_times,
	(void (*)(void))&sys_shutdown,
	(void (*)(void))&sys_ps,
	(void (*)(void))&sys_gticks,
    NULL,
    NULL,
    NULL
};
