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
 * @brief Inode module implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <errno.h>
#include <limits.h>
#include "fs.h"

/* Number of inodes per block. */
#define INODES_PER_BLOCK (BLOCK_SIZE/sizeof(struct d_inode))

/**
 * @brief In-core inodes table.
 */
PRIVATE struct inode inodes[NR_INODES];

/**
 * @brief Hash table size.
 */
#define HASHTAB_SIZE 227

/* Free inodes. */
PRIVATE struct inode *free_inodes = NULL;

/* Inodes hash table. */
PRIVATE struct inode *hashtab[HASHTAB_SIZE];

/**
 * @brief Hash function for the inode cache.
 */
#define HASH(dev, num) \
	(((dev)^(num))%HASHTAB_SIZE)

/**
 * @brief Evicts an free inode from the inode cache
 *
 * @details Searches the inode cache for a free inode and returns it.
 *
 * @returns If a free inode is found, that inode is locked and then returned.
 *          However, no there is no free inode, a #NULL pointer is returned
 *          instead.
 */
PRIVATE struct inode *inode_cache_evict(void)
{
	struct inode *ip;

	/*
	 * No free inodes.
	 * If this happens too often, it
	 * may indicate that inode cache
	 * is too small.
	 */
	if (free_inodes == NULL)
	{
		kprintf("fs: inode table overflow");
		return (NULL);
	}

	/* Remove inode from free list. */
	ip = free_inodes;
	free_inodes = free_inodes->free_next;

	ip->count++;
	inode_lock(ip);

	return (ip);
}

/**
 * @brief Inserts an inode in the inode cache.
 *
 * @details Inserts the inode pointed to by @p ip in the inode cache.
 *
 * @param ip Inode to be inserted back in the inode cache.
 *
 * @note The inode must be locked.
 */
PRIVATE void inode_cache_insert(struct inode *ip)
{
	unsigned i;

	i = HASH(ip->dev, ip->num);

	/* Insert the inode in the hash table. */
	ip->hash_next = hashtab[i];
	ip->hash_prev = NULL;
	hashtab[i] = ip;
	if (ip->hash_next != NULL)
		ip->hash_next->hash_prev = ip;
}

/**
 * @brief Removes an inode from the inode cache.
 *
 * @details Removes the inode pointed to by @p ip form the inode cache.
 *
 * @param ip Inode to be removed from the cache.
 *
 * @note The inode must be locked.
 */
PRIVATE void inode_cache_remove(struct inode *ip)
{
	/* Remove inode from the hash table. */
	if (ip->hash_prev != NULL)
		ip->hash_prev->hash_next = ip->hash_next;
	else
		hashtab[HASH(ip->dev, ip->num)] = ip->hash_next;
	if (ip->hash_next != NULL)
		ip->hash_next->hash_prev = ip->hash_prev;
}

/**
 * @brief Writes an inode to disk.
 *
 * @details Writes the inode pointed to by @p ip to disk.
 *
 * @param ip Inode to be written to disk.
 *
 * @note The inode must be locked.
 */
PRIVATE void inode_write(struct inode *ip)
{
	block_t blk;           /* Block.       */
	struct buffer *buf;    /* Buffer.      */
	struct d_inode *d_i;   /* Disk inode.  */
	struct superblock *sb; /* Super block. */

	/* Nothing to be done. */
	if (!(ip->flags & INODE_DIRTY))
		return;

	superblock_lock(sb = ip->sb);

	blk = 2 + sb->imap_blocks + sb->zmap_blocks + (ip->num - 1)/INODES_PER_BLOCK;

	/* Read chunk of disk inodes. */
	buf = bread(ip->dev, blk);
	if (buf == NULL)
	{
		kprintf("fs: failed to write inode %d to disk", ip->num);
		superblock_unlock(sb);
	}

	d_i = &(((struct d_inode *)buf->data)[(ip->num - 1)%INODES_PER_BLOCK]);

	/* Write inode to buffer. */
	d_i->i_mode = ip->mode;
	d_i->i_nlinks = ip->nlinks;
	d_i->i_uid = ip->uid;
	d_i->i_gid = ip->gid;
	d_i->i_size = ip->size;
	d_i->i_time = ip->time;
	for (unsigned i = 0; i < NR_ZONES; i++)
		d_i->i_zones[i] = ip->blocks[i];
	ip->flags &= ~INODE_DIRTY;
	buffer_dirty(buf, 1);

	brelse(buf);
	superblock_unlock(sb);
}

/**
 * @brief Reads an inode from the disk.
 *
 * @details Reads the inode with number @p num from the device @p dev.
 *
 * @param dev Device where the inode is located.
 * @param num Number of the inode that shall be read.
 *
 * @returns Upon successful completion a pointer to a in-core inode is returned.
 *          In this case, the inode is ensured to be locked. Upon failure, a
 *          #NULL pointer is returned instead.
 *
 * @note The device number must be valid.
 * @note The inode number must be valid.
 */
PRIVATE struct inode *inode_read(dev_t dev, ino_t num)
{
	block_t blk;           /* Block.         */
	struct inode *ip;      /* In-core inode. */
	struct buffer *buf;    /* Buffer.        */
	struct d_inode *d_i;   /* Disk inode.    */
	struct superblock *sb; /* Super block.   */

	/* Get superblock. */
	sb = superblock_get(dev);
	if (sb == NULL)
		goto error0;

	/* Calculate block number. */
	blk = 2 + sb->imap_blocks + sb->zmap_blocks + (num - 1)/INODES_PER_BLOCK;

	/* Read chunk of disk inodes. */
	buf = bread(dev, blk);
	if (buf == NULL)
	{
		kprintf("fs: failed to read inode %d from disk", num);
		goto error1;
	}

	d_i = &(((struct d_inode *)buf->data)[(num - 1)%INODES_PER_BLOCK]);

	/* Invalid disk inode. */
	if (d_i->i_nlinks == 0)
		goto error1;

	/* Get a free in-core inode. */
	ip = inode_cache_evict();
	if (ip == NULL)
		goto error1;

	/* Initialize in-core inode. */
	ip->mode = d_i->i_mode;
	ip->nlinks = d_i->i_nlinks;
	ip->uid = d_i->i_uid;
	ip->gid = d_i->i_gid;
	ip->size = d_i->i_size;
	ip->time = d_i->i_time;
	for (unsigned i = 0; i < NR_ZONES; i++)
		ip->blocks[i] = d_i->i_zones[i];
	ip->dev = dev;
	ip->num = num;
	ip->sb = sb;
	ip->flags &= ~(INODE_DIRTY | INODE_MOUNT | INODE_PIPE);
	ip->flags |= INODE_VALID;

	brelse(buf);
	superblock_put(sb);

	return (ip);

error1:
	superblock_put(sb);
error0:
	return (NULL);
}

/**
 * @brief Frees an inode.
 *
 * @details Frees the inode pointed to by @p ip.
 *
 * @details The inode must be locked.
 */
PRIVATE void inode_free(struct inode *ip)
{
	block_t blk;           /* Block number.           */
	struct superblock *sb; /* Underlying super block. */

	blk = (ip->num - 1)/(BLOCK_SIZE << 3);

	superblock_lock(sb = ip->sb);

	bitmap_clear(sb->imap[blk]->data, (ip->num - 1)%(BLOCK_SIZE << 3));

	sb->imap[blk]->flags |= BUFFER_DIRTY;
	if (ip->num < sb->isearch)
		sb->isearch = ip->num;
	sb->flags |= SUPERBLOCK_DIRTY;

	superblock_unlock(sb);
}

/**
 * @brief Locks an inode.
 *
 * @details Locks the inode pointed to by @p ip.
 */
PUBLIC void inode_lock(struct inode *ip)
{
	while (ip->flags & INODE_LOCKED)
		sleep(&ip->chain, PRIO_INODE);
	ip->flags |= INODE_LOCKED;
}

/**
 * @brief Unlocks an inode.
 *
 * @details Unlocks the inode pointed to by @p ip.
 *
 * @param ip Inode to be unlocked.
 *
 * @note The inode must be locked.
 */
PUBLIC void inode_unlock(struct inode *ip)
{
	wakeup(&ip->chain);
	ip->flags &= ~INODE_LOCKED;
}

/**
 * @brief Synchronizes the in-core inode table.
 *
 * @details Synchronizes the in-core inode table by flushing all valid inodes
 *          onto underlying devices.
 */
PUBLIC void inode_sync(void)
{
	/* Write valid inodes to disk. */
	for (struct inode *ip = &inodes[0]; ip < &inodes[NR_INODES]; ip++)
	{
		inode_lock(ip);

		/* Write only valid inodes. */
		if (ip->flags & INODE_VALID)
		{
			if (!(ip->flags & INODE_PIPE))
				inode_write(ip);
		}

		inode_unlock(ip);
	}
}

/**
 * @brief Truncates an inode.
 *
 * @details Truncates the inode pointed to by @p ip by freeing all underling
 *          blocks.
 *
 * @param ip Inode that shall be truncated.
 *
 * @note The inode must be locked.
 */
PUBLIC void inode_truncate(struct inode *ip)
{
	struct superblock *sb;

	superblock_lock(sb = ip->sb);

	/* Free direct zone. */
	for (unsigned j = 0; j < NR_ZONES_DIRECT; j++)
	{
		block_free(sb, ip->blocks[j], 0);
		ip->blocks[j] = BLOCK_NULL;
	}

	/* Free singly indirect zones. */
	for (unsigned j = 0; j < NR_ZONES_SINGLE; j++)
	{
		block_free(sb, ip->blocks[NR_ZONES_DIRECT + j], 1);
		ip->blocks[NR_ZONES_DIRECT + j] = BLOCK_NULL;
	}

	/* Free double indirect zones. */
	for (unsigned j = 0; j < NR_ZONES_DOUBLE; j++)
	{
		block_free(sb, ip->blocks[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j], 2);
		ip->blocks[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j] = BLOCK_NULL;
	}

	superblock_unlock(sb);

	ip->size = 0;
	inode_touch(ip);
}

/**
 * @brief Allocates an inode.
 *
 * @details Allocates an inode in the file system that is associated to the
 *          superblock pointed to by @p sb.
 *
 * @param sb Superblock where the inode shall be allocated.
 *
 * @returns Upon successful completion, a pointed to the inode is returned. In
 *          this case, the inode is ensured to be locked. Upon failure, a #NULL
 *          pointer is returned instead.
 *
 * @note The superblock must not be locked.
 *
 * @todo Use isearch.
 */
PUBLIC struct inode *inode_alloc(struct superblock *sb)
{
	ino_t num;        /* Inode number.             */
	bit_t bit;        /* Bit number in the bitmap. */
	unsigned i;       /* Number of current block.  */
	struct inode *ip; /* Inode.                    */

	superblock_lock(sb);

	/* Search for free inode. */
	for (i = 0; i < sb->imap_blocks; i++)
	{
		bit = bitmap_first_free(sb->imap[i]->data, BLOCK_SIZE);

		/* Found. */
		if (bit != BITMAP_FULL)
			goto found;
	}

	goto error0;

found:

	num = bit + i*(BLOCK_SIZE << 3) + 1;

	/*
	 * Remember disk block number to
	 * speedup next allocation.
	 */
	sb->isearch = num;

	/* Get a free in-core inode. */
	ip = inode_cache_evict();
	if (ip == NULL)
		goto error0;

	/* Allocate inode. */
	bitmap_set(sb->imap[i]->data, bit);
	sb->imap[i]->flags |= BUFFER_DIRTY;
	sb->flags |= SUPERBLOCK_DIRTY;

	/*
	 * Initialize inode.
	 * mode will be initialized later.
	 */
	ip->nlinks = 1;
	ip->uid = curr_proc->euid;
	ip->gid = curr_proc->egid;
	ip->size = 0;
	for (unsigned j = 0; j < NR_ZONES; j++)
		ip->blocks[j] = BLOCK_NULL;
	ip->dev = sb->dev;
	ip->num = num;
	ip->sb = sb;
	ip->flags &= ~(INODE_MOUNT | INODE_PIPE);
	ip->flags |= INODE_VALID;
	inode_touch(ip);

	inode_cache_insert(ip);

	superblock_unlock(sb);

	return (ip);

error0:
	superblock_unlock(sb);
	return (NULL);
}

/**
 * @brief Gets an inode.
 *
 * @details Gets the inode with number @p num from the device @p dev.
 *
 * @param dev Device where the inode is located.
 * @param num Number of the inode.
 *
 * @returns Upon successful completion, a pointer to the inode is returned. In
 *          this case, the inode is ensured to be locked. Upon failure, a #NULL
 *          pointer is returned instead.
 *
 * @note The device number must be valid.
 * @note The inode number must be valid.
 */
PUBLIC struct inode *inode_get(dev_t dev, ino_t num)
{
	struct inode *ip;

repeat:

	/* Search in the hash table. */
	for (ip = hashtab[HASH(dev, num)]; ip != NULL; ip = ip->hash_next)
	{
		/* Not found. */
		if (ip->dev != dev || ip->num != num)
			continue;

		/* Inode is locked. */
		if (ip->flags & INODE_LOCKED)
		{
			sleep(&ip->chain, PRIO_INODE);
			goto repeat;
		}

		/* Cross mount point. */
		if (ip->flags & INODE_MOUNT)
		{
			 dev = ip->dev;
			 goto repeat;
		}

		ip->count++;
		inode_lock(ip);

		return (ip);
	}

	/* Read inode. */
	ip = inode_read(dev, num);
	if (ip == NULL)
		return (NULL);

	inode_cache_insert(ip);

	return (ip);
}

/*
 * Gets a pipe inode.
 */
PUBLIC struct inode *inode_pipe(void)
{
	void *pipe;          /* Pipe page.  */
	struct inode *inode; /* Pipe inode. */

	pipe = getkpg(0);

	/* Failed to get pipe page. */
	if (pipe == NULL)
		goto error0;

	inode = inode_cache_evict();

	/* No free inode. */
	if (inode == NULL)
		goto error1;

	/* Initialize inode. */
	inode->mode = MAY_READ | MAY_WRITE | S_IFIFO;
	inode->nlinks = 0;
	inode->uid = curr_proc->uid;
	inode->gid = curr_proc->gid;
	inode->size = PAGE_SIZE;
	inode->time = CURRENT_TIME;
	inode->dev = NULL_DEV;
	inode->num = INODE_NULL;
	inode->count = 2;
	inode->flags |= ~(INODE_DIRTY | INODE_MOUNT) & (INODE_VALID | INODE_PIPE);
	inode->pipe = pipe;
	inode->head = 0;
	inode->tail = 0;

	return (inode);

error1:
	putkpg(pipe);
error0:
	return (NULL);
}

/**
 * @brief Updates the time stamp of an inode.
 *
 * @details Updates the time stamp of the inode pointed to by @p ip to current
 *          time.
 *
 * @param ip Inode to be touched.
 *
 * @note The inode must be locked.
 */
PUBLIC inline void inode_touch(struct inode *ip)
{
	ip->time = CURRENT_TIME;
	ip->flags |= INODE_DIRTY;
}

/**
 * @brief Releases a in-core inode.
 *
 * @details Releases the inc-core inode pointed to by @p ip. If its reference
 *          count drops to zero, all underlying resources are freed and then the
 *          inode is marked as invalid.
 *
 * @param ip Inode that shall be released.
 *
 * @note The inode must be valid
 * @note The inode must be locked.
 */
PUBLIC void inode_put(struct inode *ip)
{
	/* Double free. */
	if (ip->count == 0)
		kpanic("freeing inode twice");

	/* Release underlying resources. */
	if (--ip->count == 0)
	{
		/* Pipe inode. */
		if (ip->flags & INODE_PIPE)
			putkpg(ip->pipe);

		/* File inode. */
		else
		{
			/* Free underlying disk blocks. */
			if (ip->nlinks == 0)
			{
				inode_free(ip);
				inode_truncate(ip);
			}

			inode_write(ip);
			inode_cache_remove(ip);
		}

		/* Insert inode in the free list. */
		ip->free_next = free_inodes;
		free_inodes = ip;

		ip->flags &= ~INODE_VALID;
	}

	inode_unlock(ip);
}

/**
 * @brief Breaks a path
 *
 * @details Parses the path pointed to by @p pathname extracting the first
 *          path-part from it. The path-part is stored in the array pointed to
 *          by @p filename.
 *
 * @param pathname Path that shall be broken.
 * @param filename Array where the first path-part should be save.
 *
 * @returns Upon successful completion, a pointer to the second path-part is
 *          returned, so a new call to this function can be made to parse the
 *          remainder of the path. Upon failure, a #NULL pointer is returned
 *          instead.
 */
PRIVATE const char *break_path(const char *pathname, char *filename)
{
	char *p2;       /* Write pointer. */
	const char *p1; /* Read pointer.  */

	p1 = pathname;
	p2 = filename;

	/* Skip those. */
	while (*p1 == '/')
		p1++;

	/* Get file name. */
	while ((*p1 != '\0') && (*p1 != '/'))
	{
		/* File name too long. */
		if ((p2 - filename) > NAME_MAX)
			return (NULL);

		*p2++ = *p1++;
	}

	*p2 = '\0';

	return (p1);
}

/*
 * Gets inode of the topmost directory of a path.
 */
PUBLIC struct inode *inode_dname(const char *path, const char **name)
{
	dev_t dev;                   /* Current device.     */
	ino_t ent;                   /* Directory entry.    */
	struct inode *i;             /* Working inode.      */
	const char *p;               /* Current path.       */
	struct superblock *sb;       /* Working superblock. */
	char filename[NAME_MAX + 1]; /* File name.          */

	p = path;

	/* Absolute path. */
	if (*p == '/')
		i = curr_proc->root;

	/* Relative path. */
	else if (*p != '\0')
		i = curr_proc->pwd;

	/* Empty path name. */
	else
	{
		curr_proc->errno = -EINVAL;
		return (NULL);
	}

	i->count++;
	inode_lock(i);

	p = break_path((*name) = p, filename);

	/* Failed to break path. */
	if (p == NULL)
		goto error0;

	/* Parse all file path. */
	while (*p != '\0')
	{
		/* Not a directory. */
		if (!S_ISDIR(i->mode))
		{
			curr_proc->errno = -ENOTDIR;
			goto error0;
		}

again:

		/* Permission denied. */
		if (!permission(i->mode, i->uid, i->gid, curr_proc, MAY_EXEC, 0))
		{
			curr_proc->errno = -EACCES;
			goto error0;
		}

		/* Root directory reached. */
		if ((curr_proc->root->num == i->num) && (!kstrcmp(filename, "..")))
		{
			do
			{
				p = break_path((*name) = p, filename);

				if (p == NULL)
					goto error0;

			} while (!kstrcmp(filename, ".."));
		}

		ent = dir_search(i, filename);

		/* No such file or directory. */
		if (ent == INODE_NULL)
		{
			/* This path was really invalid. */
			if (*p != '\0')
			{
				curr_proc->errno = -ENOENT;
				goto error0;
			}

			/* Let someone else decide what to do. */
			break;
		}

		/* Cross mount point. */
		if ((i->num == INODE_ROOT) && (!kstrcmp(filename, "..")))
		{
			sb = i->sb;
			inode_put(i);
			i = sb->mp;
			inode_lock(i);
			i->count++;
			goto again;
		}

		dev = i->dev;
		inode_put(i);
		i = inode_get(dev, ent);

		/* Failed to get inode. */
		if (i == NULL)
			return (NULL);

		p = break_path((*name) = p, filename);

		/* Failed to break path. */
		if (p == NULL)
			goto error0;
	}

	/* Special treatment for root directory. */
	if (kstrcmp(*name, "/"))
	{
		while (**name == '/')
			(*name)++;
	}

	return (i);

error0:
	inode_put(i);
	return (NULL);
}

/*
 * Converts a path name to inode.
 */
PUBLIC struct inode *inode_name(const char *pathname)
{
	dev_t dev;           /* Device number. */
	ino_t num;           /* Inode number.  */
	const char *name;    /* File name.     */
	struct inode *inode; /* Working inode. */

	inode = inode_dname(pathname, &name);

	/* Failed to get directory inode. */
	if (inode == NULL)
		return (NULL);

	/* Special treatment for the root directory. */
	if (!kstrcmp(name,"/"))
		num = curr_proc->root->num;
	else
		num = dir_search(inode, name);

	/* File not found. */
	if (num == INODE_NULL)
	{
		inode_put(inode);
		curr_proc->errno = -ENOENT;
		return (NULL);
	}

	dev = inode->dev;
	inode_put(inode);

	return (inode_get(dev, num));
}

/**
 * @brief Initializes the inode table.
 *
 * @details Initializes the in-core inode table by marking all in-core inodes
 *          as invalid and not locked.
 */
PUBLIC void inode_init(void)
{
	kprintf("fs: initializing inode cache");

	/* Initialize inodes. */
	for (unsigned i = 0; i < NR_INODES; i++)
	{
		inodes[i].count = 0;
		inodes[i].flags = ~(INODE_LOCKED | INODE_VALID);
		inodes[i].chain = NULL;
		inodes[i].free_next = ((i + 1) < NR_INODES) ? &inodes[i + 1] : NULL;
		inodes[i].hash_next = NULL;
		inodes[i].hash_prev = NULL;
	}

	/* Initialize inode cache. */
	free_inodes = &inodes[0];
	for (unsigned i = 0; i < HASHTAB_SIZE; i++)
		hashtab[i] = NULL;
}
