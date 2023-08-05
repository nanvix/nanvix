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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/**
 * @file
 *
 * @brief Minix file system specification.
 */

#ifndef MINIX_H_
#define MINIX_H_

	#include <stdint.h>

/*============================================================================*
 *                              Block Information                             *
 *============================================================================*/

	/**
	 * @brief Log 2 of block size.
	 */
	#define BLOCK_SIZE_LOG2 10

	/**
	 * @brief Block size (in bytes).
	 */
	#define BLOCK_SIZE (1 << BLOCK_SIZE_LOG2)

	/**
	 * @brief User for block number.
	 */
	typedef uint16_t block_t;

	/**
	 * @brief Null block.
	 */
	#define BLOCK_NULL 0

/*============================================================================*
 *                           Superblock Information                           *
 *============================================================================*/

	/**
 	 * @brief Superblock magic number.
 	 */
 	#define SUPER_MAGIC 0x137f

	/**
	 * @brief In-disk superblock.
	 */
	struct d_superblock
	{
		uint16_t s_ninodes;          /**< Number of inodes.           */
		uint16_t s_nblocks;          /**< Number of blocks.           */
		uint16_t s_imap_nblocks;     /**< Number of inode map blocks. */
		uint16_t s_bmap_nblocks;     /**< Number of block map blocks. */
		uint16_t s_first_data_block; /**< Unused.                     */
		uint16_t unused1;            /**< Unused.                     */
		uint32_t s_max_size;         /**< Maximum file size.          */
		uint16_t s_magic;            /**< Magic number.               */
	} __attribute__((packed));

/*============================================================================*
 *                             Inode Information                              *
 *============================================================================*/

	/**
	 * @brief Null inode.
	 */
	#define INODE_NULL 0

	/**
	 * @brief Root inode.
	 */
	#define INODE_ROOT 1

	/**
	 * @name Number of Zones
	 *
	 * @details Number of zones in an #inode.
	 */
	/**@{*/
	#define NR_ZONES_DIRECT 7 /**< Number of direct zones.          */
	#define NR_ZONES_SINGLE 1 /**< Number of single indirect zones. */
	#define NR_ZONES_DOUBLE 1 /**< Number of double indirect zones. */
	#define NR_ZONES        9 /**< Total of zones.                  */
	/**@}*/

	/**
	 * @name Zone Index
	 *
	 * @details Index of zones in an #inode.
	 */
	/**@{*/
	#define ZONE_DIRECT                               0 /**< Direct zone.     */
	#define ZONE_SINGLE               (NR_ZONES_DIRECT) /**< Single indirect. */
	#define ZONE_DOUBLE (ZONE_SINGLE + NR_ZONES_SINGLE) /**< Double indirect. */
	/**@}*/

	/**
	 * @name Zone Dimensions
	 *
	 * @details Dimension of zones.
	 */
	/**@{*/

	/** Number of zones in a direct zone. */
	#define NR_DIRECT 1

	/** Number of zones in a single indirect zone. */
	#define NR_SINGLE (BLOCK_SIZE/sizeof(block_t))

	/** Number of zones in a double indirect zone. */
	#define NR_DOUBLE ((BLOCK_SIZE/sizeof(block_t))*NR_SINGLE)

	/**@}*/

	/**
	 * @brief Disk inode.
	 */
	struct d_inode
	{
		uint16_t i_mode;            /**< Access permissions.                  */
		uint16_t i_uid;             /**< User id of the file's owner          */
		uint32_t i_size;            /**< File size (in bytes).                */
		uint32_t i_time;            /**< Time when the file was last accessed.*/
		uint8_t i_gid;              /**< Group number of owner user.          */
		uint8_t i_nlinks;           /**< Number of links to the file.         */
		uint16_t i_zones[NR_ZONES]; /**< Zone numbers.                        */
	} __attribute__((packed));

/*============================================================================*
 *                         Directory Entry Information                        *
 *============================================================================*/

	/**
	 * @brief Maximum name on a Minix file system.
	 */
	#define MINIX_NAME_MAX 14

	/*
	 * Directory entry.
	 */
	struct d_dirent
	{
		uint16_t d_ino;              /**< File serial number. */
		char d_name[MINIX_NAME_MAX]; /**< Name of entry.      */
	} __attribute__((packed));

#endif /* MINIX_H_ */
