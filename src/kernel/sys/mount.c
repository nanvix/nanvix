/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <errno.h>

/**
 * Mounts a file system.
 *
 * @param device  Device name.
 * @param target  Target directory.
 */
PUBLIC int sys_mount(const char *device, const char *target)
{
	char *kdevice;
	char *ktarget;

	/* Get device name. */
	if ((kdevice = getname(device)) == NULL)
		return (curr_proc->errno);
	
	/* Get target directory. */
	if ((ktarget = getname(target)) == NULL)
		return (curr_proc->errno);

	kprintf("fs: mouting %s on %s", ktarget, kdevice);

	return (mount(kdevice,ktarget));
}

