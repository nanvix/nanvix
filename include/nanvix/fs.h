/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <nanvix/fs.h> - File system library.
 */

#ifndef FS_H_
#define FS_H_

#ifndef _ASM_FILE_

	#include <nanvix/config.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>
	#include <stdint.h>
	
	/* Number of zones. */
	#define NR_ZONES_DIRECT 7 /* Direct.          */
	#define NR_ZONES_SINGLE 1 /* Single indirect. */
	#define NR_ZONES_DOUBLE 1 /* Double indirect. */
	#define NR_ZONES        9 /* Total.           */
	
	/* No zone. */
	#define NO_ZONE 0
	
	/* Zone number. */
	typedef uint32_t zone_t;

/*============================================================================*
 *                             Block Buffer Library                           *
 *============================================================================*/

	/* Block size. */
	#define BLOCK_SIZE 1024
	
	/* Buffer flags. */
	#define BUFFER_DIRTY  1 /* Dirty?  */
	#define BUFFER_VALID  2 /* Valid?  */
	#define BUFFER_LOCKED 4 /* Locked? */

	/* Used for block number. */
	typedef uint32_t block_t;

	/*
	 * Block buffer.
	 */
	struct buffer
	{
		/* General information. */
		dev_t dev;   /* Device.          */
		block_t num; /* Block number.    */
		void *data;  /* Underlying data. */
		int count;   /* Reference count. */
		
		/* Status information. */
		int flags;             /* Flags (see above). */
		struct process *chain; /* Sleeping chain.    */
		
		/* Cache information. */
		struct buffer *free_next; /* Next buffer in the free list.      */
		struct buffer *free_prev; /* Previous buffer in the free list.  */
		struct buffer *hash_next; /* Next buffer in the hash table.     */
		struct buffer *hash_prev; /* Previous buffer in the hash table. */
	};
	
	/*
	 * Initializes the block buffer cache.
	 */
	EXTERN void cache_init(void);
	
	/*
	 * Synchronizes the block buffers cache.
	 */
	EXTERN void cache_sync(void);

	/*
	 * Locks a block buffer.
	 */
	EXTERN void block_lock(struct buffer *buf);

	/*
	 * Unlocks a block buffer.
	 */
	EXTERN void block_unlock(struct buffer *buf);

	/*
	 * Releases access to a block buffer.
	 */
	EXTERN void block_put(struct buffer *buf);

	/*
	 * Reads a block buffer.
	 */
	EXTERN struct buffer *block_read(dev_t dev, block_t num);

	/*
	 * Writes a block buffer.
	 */
	EXTERN void block_write(struct buffer *buf);
	
/*============================================================================*
 *                               Inode Library                                *
 *============================================================================*/
	
	/* Inode flags. */
	#define INODE_LOCKED 1 /* Locked?      */
	#define INODE_DIRTY  2 /* Dirty?       */
	#define INODE_MOUNT  4 /* Mount point? */
	#define INODE_VALID  8 /* Valid inode? */
 
	/*
	 * In-core memory inode.
	 */
	struct inode 
	{
		mode_t mode;             /* Acess permissions.                    */
		nlink_t nlinks;          /* Number of links to the file.          */
		uid_t uid;               /* User id of the file's owner           */
		gid_t gid;               /* Group number of owner user.           */
		off_t size;              /* File size (in bytes).                 */
		time_t time;             /* Time when the file was last modified. */
		zone_t zones[NR_ZONES];  /* Zone numbers.                         */
		dev_t dev;               /* Underlying device.                    */
		ino_t num;               /* Inode number.                         */
		struct superblock *sb;   /* Super block.                          */        
		int count;               /* Reference count.                      */
		unsigned flags;          /* Flags (see above).                    */
		struct inode *free_next; /* Next inode in the free list.          */
		struct inode *hash_next; /* Next inode in the hash table.         */
		struct inode *hash_prev; /* Previous unode in the hash table.     */
		struct process *chain;   /* Sleeping chain.                       */
	};
	
	/*
	 * Locks an inode.
	 */
	EXTERN void inode_lock(struct inode *i);

	/*
	 * Unlocks an inode.
	 */
	EXTERN void inode_unlock(struct inode *i);

	/*
	 * Synchronizes all inodes.
	 */
	EXTERN void inode_sync(void);

	/*
	 * Truncates an inode.
	 */
	EXTERN void inode_truncate(struct inode *i);

	/*
	 * Allocates an inode.
	 */
	EXTERN struct inode *inode_alloc(struct superblock *sb);

	/*
	 * Gets access to inode.
	 */
	EXTERN struct inode *inode_get(dev_t dev, ino_t num);

	/*
	 * Releases access to inode.
	 */
	EXTERN void inode_put(struct inode *i);

/*============================================================================*
 *                            Super Block Library                             *
 *============================================================================*/

	/* Max. inode map size. */
	#define IMAP_SIZE (HDD_SIZE/(BLOCK_SIZE*BLOCK_SIZE*8))
	
	/* Max. zone map size. */
	#define ZMAP_SIZE (HDD_SIZE/(BLOCK_SIZE*BLOCK_SIZE*8))

	/* Super block flags. */
	#define SUPERBLOCK_RDONLY 1 /* Read only?         */
	#define SUPERBLOCK_LOCKED 2 /* Locked?            */
	#define SUPERBLOCK_DIRTY  4 /* Dirty?             */
	#define SUPERBLOCK_VALID  8 /* Valid super block? */
	
	/*
	 * In-core super block.
	 */
	struct superblock
	{
		struct buffer *buf;             /* Buffer disk super block.      */
		ino_t ninodes;                  /* Number of inodes.             */
		struct buffer *imap[IMAP_SIZE]; /* Inode map.                    */
		block_t imap_blocks;            /* Number of inode map blocks.   */
		struct buffer *zmap[ZMAP_SIZE]; /* Zone map.                     */
		block_t zmap_blocks;            /* Number of zone map blocks.    */
		zone_t firstdatazone;           /* Number of first data zone.    */
		unsigned log_zone_size;         /* Log2 of blocks/zone.          */
		off_t max_size;                 /* Maximum file size.            */
		zone_t zones;                   /* Number of zones.              */
		struct inode *root;             /* Inode for root directory.     */
		struct inode *mp;               /* Inode mounted on.             */
		dev_t dev;                      /* Underlying device.            */
		int flags;                      /* Flags (see above).            */
		ino_t isearch;		            /* Inodes below this are in use. */
		zone_t zsearch;		            /* Zones below this are in use.  */
		struct process *chain;          /* Waiting chain.                */
	};
	
	/*
	 * Locks a super block.
	 */
	EXTERN void superblock_lock(struct superblock *sb);
	
	/*
	 * Unlocks a super block.
	 */
	EXTERN void superblock_unlock(struct superblock *sb);
	
	/*
	* Gets super block.
	 */
	EXTERN struct superblock *superblock_get(dev_t dev);
	
	/*
	 * Releases a super block.
	 */
	EXTERN void superblock_put(struct superblock *sb);
	
	/*
	 * Writes a super block to a device.
	 */
	EXTERN void superblock_write(struct superblock *sb);
	
	/*
	 * Reads a super block from a device.
	 */
	EXTERN struct superblock *superblock_read(dev_t dev);
	
	/*
	 * Super block table.
	 */
	EXTERN struct superblock superblocks[NR_SUPERBLOCKS];
 
/*============================================================================*
 *                                Zone Library                                *
 *============================================================================*/	
	
	/*
	 * Allocates a zone.
	 */
	EXTERN zone_t zone_alloc(struct superblock *sb);

	/*
	 * Frees a zone.
	 */
	EXTERN void zone_free(struct superblock *sb, zone_t num);

	/*
	 * Frees an indirect zone.
	 */
	EXTERN void zone_free_indirect(struct superblock *sb, zone_t num);

	/*
	 * Frees a doubly indirect zone.
	 */
	EXTERN void zone_free_dindirect(struct superblock *sb, zone_t num);
	
/*============================================================================*
 *                              File System Manager                           *
 *============================================================================*/	

	/*
	 * Initializes the file system manager.
	 */
	EXTERN void fs_init(void);
	
	/*
	 * Root device.
	 */
	EXTERN struct superblock *rootdev;
	
	/*
	 * Root directory.
	 */
	EXTERN struct inode *root;

#endif /* _ASM_FILE */

#endif /* FS_H_ */
