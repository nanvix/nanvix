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

/**
 * @file
 *
 * @brief Private file system interface.
 */

#ifndef _FS_H_
#define _FS_H_

	#include <nanvix/const.h>
	#include <nanvix/fs.h>
	#include <stdint.h>
	#include <limits.h>

/*============================================================================*
 *                              Buffer Library                                *
 *============================================================================*/

 	/**
 	 * @addtogroup Buffer
 	 */
	/**@{*/

	/**
	 * @brief Buffer flags.
	 */
	enum buffer_flags
	{
		BUFFER_DIRTY  = (1 << 0), /**< Dirty?             */
		BUFFER_VALID  = (1 << 1), /**< Valid?             */
		BUFFER_LOCKED = (1 << 2), /**< Locked?            */
		BUFFER_SYNC   = (1 << 3)  /**< Synchronous write? */
	};

	/**
	 * @brief Block buffer.
	 */
	struct buffer
	{
		/**
		 * @name General information
		 */
		/**@{*/
		dev_t dev;      /**< Device.          */
		block_t num;    /**< Block number.    */
		void *data;     /**< Underlying data. */
		unsigned count; /**< Reference count. */
		/**@}*/

		/**
		 * @name Status information
		 */
		/**@{*/
		enum buffer_flags flags; /**< Flags.          */
		struct process *chain;   /**< Sleeping chain. */
		/**@}*/

		/**
		 * @name Cache information.
		 */
		/**@{*/
		struct buffer *free_next; /**< Next buffer in the free list.      */
		struct buffer *free_prev; /**< Previous buffer in the free list.  */
		struct buffer *hash_next; /**< Next buffer in the hash table.     */
		struct buffer *hash_prev; /**< Previous buffer in the hash table. */
		/**@}*/
	};

	/**@}*/

	/* Forward definitions. */
	EXTERN void binit(void);

/*============================================================================*
 *                               Inode Library                                *
 *============================================================================*/

	/* Forward definitions. */
	EXTERN void inode_init(void);

/*============================================================================*
 *                            Super Block Library                             *
 *============================================================================*/

 	/**
 	 * @addtogroup Superblock
 	 */
 	/**@{*/

	/**
	 * @brief Maximum inode map size (in bytes).
	 */
	#define IMAP_SIZE (HDD_SIZE/(BLOCK_SIZE*BLOCK_SIZE*8))

	/**
	 * @brief Maximum zone map size (in bytes).
	 */
	#define ZMAP_SIZE (HDD_SIZE/(BLOCK_SIZE*BLOCK_SIZE*8))

	/**
	 * @brief Superblock flags.
	 */
	enum superblock_flags
	{
		SUPERBLOCK_RDONLY = (1 << 0), /**< Read only?        */
		SUPERBLOCK_LOCKED = (1 << 1), /**< Locked?           */
		SUPERBLOCK_DIRTY  = (1 << 2), /**< Dirty?            */
		SUPERBLOCK_VALID  = (1 << 3)  /**< Valid superblock? */
	};

	/**
	 * @brief In-core superblock.
	 */
	struct superblock
	{
		unsigned count;                 /**< Reference count.              */
		struct buffer *buf;             /**< Buffer disk superblock.       */
		ino_t ninodes;                  /**< Number of inodes.             */
		struct buffer *imap[IMAP_SIZE]; /**< Inode map.                    */
		block_t imap_blocks;            /**< Number of inode map blocks.   */
		struct buffer *zmap[ZMAP_SIZE]; /**< Zone map.                     */
		block_t zmap_blocks;            /**< Number of zone map blocks.    */
		block_t first_data_block;       /**< First data block.             */
		off_t max_size;                 /**< Maximum file size.            */
		block_t zones;                  /**< Number of zones.              */
		struct inode *root;             /**< Inode for root directory.     */
		struct inode *mp;               /**< Inode mounted on.             */
		dev_t dev;                      /**< Underlying device.            */
		enum superblock_flags flags;    /**< Flags.                        */
		ino_t isearch;		            /**< Inodes below this are in use. */
		block_t zsearch;		        /**< Zones below this are in use.  */
		struct process *chain;          /**< Waiting chain.                */
	};

	/**@}*/

#endif /* _FS_H_ */
