/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/fs.h - File system private library.
 */

#ifndef _FS_H_
#define _FS_H_

	#include <nanvix/const.h>
	#include <nanvix/fs.h>
	#include <stdint.h>
	#include <limits.h>	
	
	/*
	 * Initializes inodes.
	 */
	EXTERN void inode_init(void);
	
/*============================================================================*
 *                               Inode Library                                *
 *============================================================================*/
 
	/*
	 * Disk inode.
	 */
	struct d_inode 
	{
		uint16_t i_mode;            /* Acess permissions.                    */
		uint16_t i_uid;             /* User id of the file's owner           */
		uint32_t i_size;            /* File size (in bytes).                 */
		uint32_t i_time;            /* Time when the file was last accessed. */
		uint8_t i_gid;              /* Group number of owner user.           */
		uint8_t i_nlinks;           /* Number of links to the file.          */
		uint16_t i_zones[NR_ZONES]; /* Zone numbers.                         */
	};
	
	/*
	 * Initializes inodes.
	 */
	EXTERN void inode_init(void);

/*============================================================================*
 *                            Super Block Library                             *
 *============================================================================*/
 	
 	/* Super block magic number. */
 	#define SUPER_MAGIC 0x137f
 	
	/*
	 * Disk super block.
	 */
	struct d_superblock
	{
		uint16_t s_ninodes;          /* Number of inodes.           */
		uint16_t s_nblocks;          /* Number of blocks.           */
		uint16_t s_imap_nblocks;     /* Number of inode map blocks. */
		uint16_t s_bmap_nblocks;     /* Number of zone map blocks.  */
		uint16_t s_first_data_block; /* Unused.                     */
		uint16_t unused1;            /* Unused.                     */
		uint32_t s_max_size;         /* Maximum file size.          */
		uint16_t s_magic;            /* Magic number.               */
	};

#endif /* _FS_H_ */
