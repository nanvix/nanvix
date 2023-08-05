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

#ifndef CONFIG_H_
#define CONFIG_H_

	/* Machine configuration. */
	#define MACHINE_NAME "valhalla" /* Machine name.*/
	#define CPU                i386 /* Cpu model.                 */
	#define MEMORY_SIZE   0x1000000 /* Memory size (in bytes).    */
	#define HDD_SIZE      0x2000000 /* Hard disk size (in bytes). */
	#define SWP_SIZE      0x1000000 /* Swap disk size (in bytes). */
	#define KEYBOARD_US           1 /* US International keyboard. */

	/* Kernel configuration. */
	#define MULTIUSER              0 /* Multiuser support?              */
	#define KERNEL_VERSION     "1.2" /* Kernel version.                 */
	#define PROC_MAX              64 /* Maximum number of process.      */
	#define PROC_SIZE_MAX  0x4000000 /* Maximum process size.           */
	#define RAMDISK_SIZE    0x400000 /* RAM disks size.                 */
	#define INITRD_SIZE      0x80000 /* Init RAM disk size.             */
	#define NR_INODES           1024 /* Number of in-core inodes.       */
	#define NR_SUPERBLOCKS         4 /* Number of in-core super blocks. */
	#define ROOT_DEV          0x0101 /* Root device number.             */
	#define SWAP_DEV          0x0101 /* Swap device number.             */
	#define NR_FILES             256 /* Number of opened files.         */
	#define NR_REGIONS           128 /* Number of memory regions.       */
	#define NR_BUFFERS           256 /* Number of block buffers.        */

#endif /* CONFIG_H_ */
