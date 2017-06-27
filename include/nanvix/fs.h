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
 * @brief Public file system interface.
 */
 
#ifndef NANVIX_FS_H_
#define NANVIX_FS_H_
	
	/* General file permissions. */
	#define MAY_READ  (S_IRUSR | S_IRGRP | S_IROTH)     /* May read.        */
	#define MAY_WRITE (S_IWUSR | S_IWGRP | S_IWOTH)     /* May write.       */
	#define MAY_EXEC  (S_IXUSR | S_IXGRP | S_IXOTH)     /* May exec/search. */
	#define MAY_ALL   (MAY_READ | MAY_WRITE | MAY_EXEC) /* May anything.    */

#ifndef _ASM_FILE_

	#include <fs/minix.h>
	#include <nanvix/config.h>
	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/stat.h>
	#include <sys/types.h>
	#include <stdint.h>
	#include <ustat.h>
	#include <sys/sem.h>


/*============================================================================*
 *                              Block Buffer Library                          *
 *============================================================================*/
 
	/**
	 * @defgroup Buffer Buffer Module
	 */
	/**@{*/
 
	/**
	 * @brief Opaque pointer to a block buffer.
	 */
	typedef struct buffer * buffer_t;
	
	/**
	 * @brief Opaque pointer to a constant buffer.
	 */
	typedef const struct buffer * const_buffer_t;
	
	/* Forward definitions. */
	EXTERN void bsync(void);
	EXTERN void blklock(buffer_t);
	EXTERN void blkunlock(buffer_t);
	EXTERN void brelse(buffer_t);
	EXTERN buffer_t bread(dev_t, block_t);
	EXTERN void bwrite(buffer_t);
	EXTERN void buffer_dirty(buffer_t, int);
	EXTERN void *buffer_data(const_buffer_t);
	EXTERN dev_t buffer_dev(const_buffer_t);
	EXTERN block_t buffer_num(const_buffer_t);
	EXTERN int buffer_is_sync(const_buffer_t);
	
	/**@}*/
	
/*============================================================================*
 *                               Inode Library                                *
 *============================================================================*/
	
	/**
	 * @defgroup Inode Inode Module
	 */
	/**@{*/
	
	/**
	 * @brief Inode flags.
	 */
	enum inode_flags
	{
		INODE_LOCKED = (1 << 0), /**< Locked?      */
		INODE_DIRTY  = (1 << 1), /**< Dirty?       */
		INODE_MOUNT  = (1 << 2), /**< Mount point? */
		INODE_VALID  = (1 << 3), /**< Valid inode? */
		INODE_PIPE   = (1 << 4)  /**< Pipe inode?  */
	};
	 
	/**
	 * @brief In-core inode.
	 */
	struct inode 
	{
		mode_t mode;              /**< Access permissions.                   */
		nlink_t nlinks;           /**< Number of links to the file.          */
		uid_t uid;                /**< User id of the file's owner           */
		gid_t gid;                /**< Group number of owner user.           */
		off_t size;               /**< File size (in bytes).                 */
		time_t time;              /**< Time when the file was last accessed. */
		block_t blocks[NR_ZONES]; /**< Zone numbers.                         */
		dev_t dev;                /**< Underlying device.                    */
		ino_t num;                /**< Inode number.                         */
		struct superblock *sb;    /**< Superblock.                           */
		unsigned count;           /**< Reference count.                      */
		enum inode_flags flags;   /**< Flags.                                */
		char *pipe;               /**< Pipe page.                            */
		off_t head;               /**< Pipe head.                            */
		off_t tail;               /**< Pipe tail.                            */
		struct inode *free_next;  /**< Next inode in the free list.          */
		struct inode *hash_next;  /**< Next inode in the hash table.         */
		struct inode *hash_prev;  /**< Previous inode in the hash table.     */
		struct process *chain;    /**< Sleeping chain.                       */
	};
	
	/**@}*/
	
	/* Forward definitions. */
	EXTERN void inode_touch(struct inode *);
	EXTERN void inode_lock(struct inode *);
	EXTERN void inode_unlock(struct inode *);
	EXTERN void inode_sync(void);
	EXTERN void inode_truncate(struct inode *);
	EXTERN struct inode *inode_alloc(struct superblock *);
	EXTERN struct inode *inode_get(dev_t dev, ino_t);
	EXTERN void inode_put(struct inode *);
	EXTERN struct inode *inode_dname(const char *, const char **);
	EXTERN struct inode *inode_name(const char *);
	EXTERN struct inode *inode_pipe(void);
	EXTERN int mount (char*, char*);
	EXTERN int unmount (char*, char*);
	EXTERN struct inode *inode_semaphore(const char* name, int mode);

/*============================================================================*
 *                            Super Block Library                             *
 *============================================================================*/

	/**
	 * @defgroup Superblock Superblock Module
	 */
	/**@{*/

	/**
	 * @brief Opaque pointer to a in-core superblock.
	 */
	typedef struct superblock * superblock_t;
	
	/**@}*/
	
	/* Forward definitions. */
	EXTERN void superblock_init(void);
	EXTERN void superblock_lock(superblock_t);
	EXTERN void superblock_unlock(superblock_t);
	EXTERN superblock_t superblock_get(dev_t);
	EXTERN void superblock_put(superblock_t);
	EXTERN superblock_t superblock_read(dev_t);
	EXTERN void superblock_stat(superblock_t, struct ustat *);
	EXTERN void superblock_sync(void);
	EXTERN block_t block_map(struct inode *, off_t, int);
	EXTERN void block_free(struct superblock *, block_t, int);
	
/*============================================================================*
 *                              File System Manager                           *
 *============================================================================*/

	/**
	 * @brief File.
	 */
	struct file
	{
		int oflag;           /**< Open flags.                   */
		int count;           /**< Reference count.              */
		off_t pos;           /**< Read/write cursor's position. */
		struct inode *inode; /**< Underlying inode.             */
	};

	/* Forward definitions. */
	EXTERN void fs_init(void);
	EXTERN int permission(mode_t, uid_t, gid_t, struct process *, mode_t, int);
	EXTERN char *getname(const char *);
	EXTERN void putname(char *);
	EXTERN int getfildes(void);
	EXTERN struct file *getfile(void);
	EXTERN void do_close(int);
	EXTERN int dir_add(struct inode *, struct inode *, const char *);
	EXTERN ino_t dir_search(struct inode *, const char *);
	EXTERN int dir_remove(struct inode *, const char *);
	EXTERN ssize_t file_read(struct inode *, void *, size_t, off_t);
	EXTERN ssize_t file_write(struct inode *, const void *, size_t, off_t);
	EXTERN ssize_t pipe_read(struct inode *, char *, size_t);
	EXTERN ssize_t pipe_write(struct inode *, const char *, size_t);
	
	EXTERN struct inode *do_creat(struct inode *d, const char *name, mode_t mode, int oflag);
	EXTERN struct inode *do_open(const char *path, int oflag, mode_t mode);

	/* Forward definitions. */
	EXTERN struct inode *root;
	EXTERN struct superblock *rootdev;
	EXTERN struct file filetab[NR_FILES];

#endif /* _ASM_FILE */

#endif /* NANVIX_FS_H_ */
