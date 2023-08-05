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
	EXTERN void inode_touch(struct inode *i);
	EXTERN void inode_lock(struct inode *i);
	EXTERN void inode_unlock(struct inode *i);
	EXTERN void inode_sync(void);
	EXTERN void inode_truncate(struct inode *i);
	EXTERN struct inode *inode_alloc(struct superblock *sb);
	EXTERN struct inode *inode_get(dev_t dev, ino_t num);
	EXTERN void inode_put(struct inode *i);
	EXTERN struct inode *inode_dname(const char *path, const char **name);
	EXTERN struct inode *inode_name(const char *pathname);
	EXTERN struct inode *inode_pipe(void);

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

#endif /* NANVIX_FS_H_ */
