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

/*============================================================================*
 *                              Bitmap Library                                *
 *============================================================================*/
 
	/* Bit number. */
	typedef int32_t bit_t;
	
	/*
	 * Initializes inodes.
	 */
	EXTERN void inode_init(void);
	
	/* Bitmap is full. */
	#define BITMAP_FULL -1
	
	/* Bitmap operators. */
	#define IDX(a) ((a) >> 5)   /* Returns the index of the bit.  */
	#define OFF(a) ((a) & 0x1F) /* Returns the offset of the bit. */
	
	/*
	 * Sets a bit in a bitmap.
	 */
	#define bitmap_set(bmap, pos) \
		(((uint32_t *)(bmap))[IDX(pos)] |= (0x1 << OFF(pos)))
	
	/*
	 * Clears a bit in a bitmap.
	 */
	#define bitmap_clear(bmap, pos) \
		(((uint32_t *)(bmap))[IDX(pos)] &= ~(0x1 << OFF(pos)))
		
	/*
	 * Finds the first free bit in a bitmap.
	 */
	EXTERN bit_t bitmap_first_free(uint32_t *bitmap, size_t size);
	
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
		uint8_t i_gid;             /* Group number of owner user.           */
		uint8_t i_nlinks;          /* Number of links to the file.          */
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
		uint16_t s_ninodes;       /* Number of inodes.           */
		uint16_t s_nzones;        /* Number of zones.            */
		uint16_t s_imap_blocks;   /* Number of inode map blocks. */
		uint16_t s_zmap_blocks;   /* Number of zone map blocks.  */
		uint16_t s_firstdatazone; /* Number of first data zone.  */
		uint16_t s_log_zone_size; /* Log2 of blocks/zone.        */
		uint32_t s_max_size;      /* Maximum file size.          */
		uint16_t s_magic;         /* Magic number.               */
	};
	
	/*
	 * Initialize super blocks.
	 */
	EXTERN void superblock_init(void);

#endif /* _FS_H_ */
