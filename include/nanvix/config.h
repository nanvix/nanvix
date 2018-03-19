/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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
	
	/**
	 * @brief Machine configuration.
	 */
	/**@{*/
	#define MACHINE_NAME "valhalla" /**< Machine name.              */
	#define CPU                i386 /**< Cpu model.                 */
	#define CPU_CLOCK      20000000 /**< Clock 20MHz.               */
	#define MEMORY_SIZE   0x4000000 /**< Memory size (in bytes).    */
	#define HDD_SIZE      0x2000000 /**< Hard disk size (in bytes). */
	/**@}*/
	
	/**
	 * @brief Kernel configuration.
	 */
	/**@{*/
	#define MULTIUSER                    0 /**< Multiuser support?                 */
	#define KERNEL_VERSION           "2.0" /**< Kernel version.                    */
	#define PROC_MAX                    64 /**< Maximum number of process.         */
	#define PROC_SIZE_MAX  (MEMORY_SIZE/8) /**< Maximum process size.              */
	#define RAMDISK_SIZE          0x400000 /**< RAM disks size.                    */
	#define INITRD_SIZE           0x140000 /**< Init RAM disk size.                */
	#define NR_INODES                 1024 /**< Number of in-core inodes.          */
	#define NR_SUPERBLOCKS               4 /**< Number of in-core super blocks.    */
	#define ROOT_DEV                0x0001 /**< Root device number.                */
	#define NR_FILES                   256 /**< Number of opened files.            */
	#define NR_REGIONS                 128 /**< Number of memory regions.          */
	#define NR_BUFFERS                 256 /**< Number of block buffers.           */
	#define NR_MOUNTING_POINT           64 /**< Maximum nunber of mounting points. */
	#define DEBUG_MAX                   64 /**< Maximum number of debug functions. */
	/**@}*/
	
#endif /* CONFIG_H_ */
