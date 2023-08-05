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

#ifndef RAMDISK_H_
#define RAMDISK_H_

	#include <nanvix/const.h>
	#include <nanvix/hal.h>

	/*
	 * DESCRIPTION:
	 *   The ramdisk_init() function initializes the RAM disk device driver.
	 *
	 * RETURN VALUE:
	 *   The ramdisk_init() has no return value.
	 *
	 * ERROS:
	 *   No errors are defined.
	 */
	EXTERN void ramdisk_init(void);

#endif /* RAMDISK_H_ */
