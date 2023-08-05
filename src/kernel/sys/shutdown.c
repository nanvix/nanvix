/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <dev/tty.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include <nanvix/syscall.h>
#include <errno.h>
#include <signal.h>

/**
 * @brief Shutdowns the system.
 */
PUBLIC int sys_shutdown(void)
{
	/* Not allowed. */
	if (!IS_SUPERUSER(curr_proc))
		return (-EPERM);

	shutting_down = 1;

	cdev_ioctl(kout, TTY_CLEAR, 0);
	kprintf("system is going to shutdown NOW");
	kprintf("synchronizing data...");
	sys_sync();
	kprintf("asking process to terminate...");
	sys_kill(-1, SIGKILL);

	return (0);
}
