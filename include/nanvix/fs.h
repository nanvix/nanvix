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
	#include <sys/stat.h>
	#include <sys/types.h>
	#include <stdint.h>

/*============================================================================*
 *                              Block Buffer Library                          *
 *============================================================================*/

	/* Null block. */
	#define BLOCK_NULL 0

	/* Block size (in bytes). */
	#define BLOCK_SIZE 1024
	
	/* Buffer flags. */
	#define BUFFER_DIRTY  1 /* Dirty?  */
	#define BUFFER_VALID  2 /* Valid?  */
	#define BUFFER_LOCKED 4 /* Locked? */

	/* Used for block number. */
	typedef uint16_t block_t;

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
	EXTERN void binit(void);
	
	/*
	 * Synchronizes the block buffers cache.
	 */
	EXTERN void bsync(void);

	/*
	 * Locks a block buffer.
	 */
	EXTERN void blklock(struct buffer *buf);

	/*
	 * Unlocks a block buffer.
	 */
	EXTERN void blkunlock(struct buffer *buf);

	/*
	 * Releases access to a block buffer.
	 */
	EXTERN void brelse(struct buffer *buf);

	/*
	 * Reads a block buffer.
	 */
	EXTERN struct buffer *bread(dev_t dev, block_t num);

	/*
	 * Writes a block buffer.
	 */
	EXTERN void bwrite(struct buffer *buf);
	
/*============================================================================*
 *                               Inode Library                                *
 *============================================================================*/
		
	/* Number of zones. */
	#define NR_ZONES_DIRECT 7 /* Direct.          */
	#define NR_ZONES_SINGLE 1 /* Single indirect. */
	#define NR_ZONES_DOUBLE 1 /* Double indirect. */
	#define NR_ZONES        9 /* Total.           */
	
	/* Zone indexes. */
	#define ZONE_DIRECT                               0 /* Direct zone.     */ 
	#define ZONE_SINGLE               (NR_ZONES_DIRECT) /* Single indirect. */
	#define ZONE_DOUBLE (ZONE_SINGLE + NR_ZONES_SINGLE) /* Double indirect. */
	
	/* Number of zones in a direct zone. */
	#define NR_DIRECT 1

	/* Number of zones in a single indirect zone. */
	#define NR_SINGLE (BLOCK_SIZE/sizeof(block_t))
	
	/* Number of zones in a double indirect zone. */
	#define NR_DOUBLE ((BLOCK_SIZE/sizeof(block_t))*NR_SINGLE)
	
	/* No inode. */
	#define INODE_NULL 0
	
	/* Root inode. */
	#define INODE_ROOT 0
	
	/* Inode flags. */
	#define INODE_LOCKED 0x01 /* Locked?      */
	#define INODE_DIRTY  0x02 /* Dirty?       */
	#define INODE_MOUNT  0x04 /* Mount point? */
	#define INODE_VALID  0x08 /* Valid inode? */
	#define INODE_PIPE   0x10 /* Pipe inode?  */
	 
	/*
	 * In-core memory inode.
	 */
	struct inode 
	{
		mode_t mode;              /* Acess permissions.                    */
		nlink_t nlinks;           /* Number of links to the file.          */
		uid_t uid;                /* User id of the file's owner           */
		gid_t gid;                /* Group number of owner user.           */
		off_t size;               /* File size (in bytes).                 */
		time_t time;              /* Time when the file was last modified. */
		block_t blocks[NR_ZONES]; /* Zone numbers.                         */
		dev_t dev;                /* Underlying device.                    */
		ino_t num;                /* Inode number.                         */
		struct superblock *sb;    /* Super block.                          */
		int count;                /* Reference count.                      */
		unsigned flags;           /* Flags (see above).                    */
		char *pipe;               /* Pipe page.                            */
		off_t head;               /* Pipe head.                            */
		off_t tail;               /* Pipe tail.                            */
		struct inode *free_next;  /* Next inode in the free list.          */
		struct inode *hash_next;  /* Next inode in the hash table.         */
		struct inode *hash_prev;  /* Previous unode in the hash table.     */
		struct process *chain;    /* Sleeping chain.                       */
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
	
	/*
	 * Gets inode of the topmost directory of a path.
	 */
	EXTERN struct inode *inode_dname(const char *path, const char **name);
	
	/*
	 * Converts pathname to inode.
	 */
	EXTERN struct inode *inode_name(const char *pathname);
	
	/*
	 * Gets a pipe inode.
	 */
	EXTERN struct inode *inode_pipe(void);

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
		int count;                      /* Reference count.              */
		struct buffer *buf;             /* Buffer disk super block.      */
		ino_t ninodes;                  /* Number of inodes.             */
		struct buffer *imap[IMAP_SIZE]; /* Inode map.                    */
		block_t imap_blocks;            /* Number of inode map blocks.   */
		struct buffer *zmap[ZMAP_SIZE]; /* Zone map.                     */
		block_t zmap_blocks;            /* Number of zone map blocks.    */
		block_t first_data_block;       /* First data block.             */
		off_t max_size;                 /* Maximum file size.            */
		block_t zones;                  /* Number of zones.              */
		struct inode *root;             /* Inode for root directory.     */
		struct inode *mp;               /* Inode mounted on.             */
		dev_t dev;                      /* Underlying device.            */
		unsigned flags;                 /* Flags (see above).            */
		ino_t isearch;		            /* Inodes below this are in use. */
		block_t zsearch;		        /* Zones below this are in use.  */
		struct process *chain;          /* Waiting chain.                */
	};
	
	/**
	 * @brief Initializes the super block table.
	 */
	EXTERN void superblock_init(void);
	
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
	 * Synchronizes super blocks.
	 */
	EXTERN void superblock_sync(void);
	
	/*
	 * Super block table.
	 */
	EXTERN struct superblock superblocks[NR_SUPERBLOCKS];
	
/*============================================================================*
 *                                  Block Library                             *
 *============================================================================*/
	
	/*
	 * Maps a file byte offset in a block number.
	 */
	EXTERN block_t block_map(struct inode *inode, off_t off, int create);

	/*
	 * Frees a disk block.
	 */
	EXTERN void block_free(struct superblock *sb, block_t num, int lvl);
	
/*============================================================================*
 *                              File System Manager                           *
 *============================================================================*/

	/* General file permissions. */
	#define MAY_READ  (S_IRUSR | S_IRGRP | S_IROTH)     /* May read.        */
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)     /* May write.       */
	#define MAY_EXEC  (S_IXUSR | S_IXGRP | S_IXOTH)     /* May exec/search. */
	#define MAY_ALL   (MAY_READ | MAY_WRITE | MAY_EXEC) /* May anything.    */

	/*
	 * File.
	 */
	struct file
	{
		int oflag;           /* Open flags.                   */
		int count;           /* Reference count.              */
		off_t pos;           /* Read/write cursor's position. */
		struct inode *inode; /* Underlying inode.             */
	};

	
	/*
	 * File table.
	 */
	EXTERN struct file filetab[NR_FILES];

	/*
	 * Initializes the file system manager.
	 */
	EXTERN void fs_init(void);
	
	/*
	 * Checks permissions on a file
	 */
	EXTERN int permission(mode_t mode, uid_t uid, gid_t gid, struct process *proc, mode_t mask, int oreal);
	
	/*
	 * Gets a user file name
	 */
	EXTERN char *getname(const char *name);
	
	/*
	 * Puts back a user file name.
	 */
	EXTERN void putname(char *name);
	
	/*
	 * Gets an empty file descriptor table entry.
	 */
	EXTERN int getfildes(void);
	
	/*
	 * Gets an empty file table entry.
	 */
	EXTERN struct file *getfile(void);
	
	/*
	 * Closes a file.
	 */
	EXTERN void do_close(int fd);
	
	/*
	 * Adds an entry to a directory.
	 */
	EXTERN int dir_add
	(struct inode *dinode, struct inode *inode, const char *name);
	
	/*
	 * Searchs for a file in a directory.
	 */
	EXTERN ino_t dir_search(struct inode *i, const char *filename);
	
	/*
	 * Removes an entry from a directory.
	 */
	EXTERN int dir_remove(struct inode *dinode, const char *filename);
	
	/*
	 * Reads from a regular file.
	 */
	EXTERN ssize_t file_read(struct inode *i, void *buf, size_t n, off_t off);
	
	/*
	 * Writes to a regular file.
	 */
	EXTERN ssize_t file_write(struct inode *i, const void *buf, size_t n, off_t off);
	
	/*
	 * Reads data from a pipe.
	 */
	EXTERN ssize_t pipe_read(struct inode *inode, char *buf, size_t n);
	
	/*
	 * Writes data to a pipe.
	 */
	EXTERN ssize_t pipe_write(struct inode *inode, const char *buf, size_t n);
	
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
