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

#ifndef USTAT_H_
#define USTAT_H_
#ifndef _ASM_FILE_

	#include <sys/types.h>

	/*
	 * File system statistics.
	 */
	struct ustat
	{
		daddr_t f_tfree; /* Total free blocks.     */
		ino_t f_tinode;  /* Number of free inodes. */
		char f_fname[6]; /* File system name.      */
		char f_fpack[6]; /* File system pack name. */
	};

	/*
	 * Gets file system statistics.
	 */
	extern int ustat(dev_t dev, struct ustat *ubuf);

#endif /* _ASM_FILE_ */
#endif /* USTAT_H_ */
