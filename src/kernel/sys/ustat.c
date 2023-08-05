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
#include <nanvix/fs.h>
#include <nanvix/mm.h>
#include <errno.h>
#include <ustat.h>

/**
 * @brief Gets file system statistics.
 *
 * @details Gets statistics about the file system that is mounted on the device
 *          dev, and stores information in the buffer pointed to by ubuf.
 *
 * @param dev  Device number where the file system is mounted.
 * @param ubuf Location where file system information shall be dumped.
 *
 * @returns Upon successful completion, zero is returned. Upon failure, a
 *          negative error number is returned instead.
 */
PUBLIC int sys_ustat(dev_t dev, struct ustat *ubuf)
{
	struct superblock *sb;

	/* Valid buffer. */
	if (!chkmem(ubuf, sizeof(struct ustat), MAY_WRITE))
		return (-EINVAL);

	sb = superblock_get(dev);

	/* Not a mounted file system. */
	if (sb == NULL)
		return (-ENODEV);

	superblock_stat(sb, ubuf);

	superblock_put(sb);

	return (0);
}
