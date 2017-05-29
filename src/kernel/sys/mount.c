/*
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

/*TODO enlever les includes inutiles*/
#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <errno.h>

/*
 * mount -t type device destination_dir
 * TODO rajouter le type 
 */
PUBLIC int sys_mount(const char *device, const char *destination_dir)
{
	char *kdevice;
	char *kdest;
	kprintf("Mount, fonction not implemented yet");
	if ((kdevice = getname(device)) == NULL)
		return (curr_proc->errno);
	if ((kdest = getname(destination_dir)) == NULL)
		return (curr_proc->errno);

	kprintf(" You are tring to mount th device %s sur le directory %s\n", kdevice, kdest);
	return 0; 
}
