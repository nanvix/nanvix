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

#ifndef UTSNAME_H_
#define UTSNAME_H_
#ifndef _ASM_FILE_

	/* Length of strings in utsname structure */
	#define _UTSNAME_LENGTH 9

	/*
	 * System name structure.
	 */
	struct utsname
	{
		char sysname[_UTSNAME_LENGTH];  /* Operating system name. */
		char nodename[_UTSNAME_LENGTH]; /* Network node name.     */
		char release[_UTSNAME_LENGTH];  /* Kernel release.        */
		char version[_UTSNAME_LENGTH];  /* Kernel version.        */
		char machine[_UTSNAME_LENGTH];  /* Hardware name.         */
	};

	/*
	 * Gets the name of the current system.
	 */
	extern int uname(struct utsname *name);

#endif /* _ASM_FILE_ */
#endif /* UTSNAME_H_ */
