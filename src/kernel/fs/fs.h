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

	
	/* Forward definitions. */
	EXTERN void buffer_share(struct buffer *);
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

	typedef struct superblock superblock;

	/**
	 *@brief In-core superblock operation
	 */
	struct super_operations {
		int (*inode_read) (dev_t, ino_t,struct inode *);
		void (*inode_write) (struct inode *);
		void (*inode_free) (struct inode *);
		void (*inode_truncate) (struct inode *);
		int (*inode_alloc) (struct superblock*, struct inode *);
		int (*notify_change) (int flags, struct inode *);
		void (*put_inode) (struct inode *);
		void (*put_super) (struct superblock *);
		void (*write_super) (struct superblock *);
		void (*remount_fs) (void);
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
		struct super_operations *so;  /**< Super operation of filesystem */
	};

/*============================================================================*
 *                       Virtual File System  Library                         *
 *============================================================================*/
	
 	/**
	 * @brief File system of the virtual file system.
	 */
	struct file_system_type {
		struct super_block *(*read_super) (dev_t); 	/**< Fonction to access the superblock of the file system 	*/
		struct super_operations *so; 				/**< stucture of the fonction of the file system 			*/
		char *name; 								/**< Name of the file system 								*/
	};

	/**
	 * @brief Mounting point.
	 */
	struct mountingPoint {
		char * device;			/**< Name of the devices							*/
		char * mountPoint;		/**< Mount point 									*/
		int no_inode_mount; 	/**< Number of the inode of the mounting point 		*/
		int no_inode_root_fs;	/**< Number of the inode root of the file system 	*/
	};

	/**
	 * @brief Number of the file system.
	 */
	#define MINIX 0
	
	/**
	 * @brief Maximum nunber of file system.
	 */
	#define NR_FILE_SYSTEM 1

	/**
	 * @brief Function too register file system in the virtual file system .
	 */
	PUBLIC int fs_register( int nb , struct file_system_type * fs );

	/**@}*/

#endif /* _FS_H_ */
