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

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/utsname.h>
#include <errno.h>

/*
 * Internal uname().
 */
PRIVATE void do_uname(struct utsname *name)
{
	/* Fill up utsname structure. */
	kstrncpy(name->sysname, SYSNAME, _UTSNAME_LENGTH);
	kstrncpy(name->nodename, NODENAME, _UTSNAME_LENGTH);
	kstrncpy(name->release, RELEASE, _UTSNAME_LENGTH);
	kstrncpy(name->version, VERSION, _UTSNAME_LENGTH);
	kstrncpy(name->machine, MACHINE, _UTSNAME_LENGTH);
}

/*
 * Gets the name of the current system.
 */
PUBLIC int sys_uname(struct utsname *name)
{
	/* Invalid buffer. */
	if (!chkmem(name, sizeof(struct utsname), MAY_WRITE))
		return (-EINVAL);

	do_uname(name);

	return (0);
}
