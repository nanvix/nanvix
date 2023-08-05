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

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include "fs.h"

/**
 * @brief Searches for a directory entry.
 *
 * @details Searches for a directory entry named @p filename in the directory
 *          pointed to be @p dip. If @p create is not zero and such file entry
 *          does not exist, the entry is created.
 *
 * @param dip      Directory where the directory entry shall be searched.
 * @param filename Name of the directory entry that shall be searched.
 * @param buf      Buffer where the directory entry is loaded.
 * @param create   Create directory entry?
 *
 * @returns Upon successful completion, the directory entry is returned. In this
 *          case, @p buf is set to point to the (locked) buffer associated to
 *          the requested directory entry. However, upon failure, a #NULL
 *          pointer is returned instead.
 *
 * @note @p dip must be locked.
 * @note @p filename must point to a valid location.
 * @note @p buf must point to a valid location
 */
PRIVATE struct d_dirent *dirent_search
(struct inode *dip, const char *filename, struct buffer **buf, int create)
{
	int i;              /* Working directory entry index.       */
	int entry;          /* Index of first free directory entry. */
	block_t blk;        /* Working block number.                */
	int nentries;       /* Number of directory entries.         */
	struct d_dirent *d; /* Directory entry.                     */

	nentries = dip->size/sizeof(struct d_dirent);

	/* Search from very first block. */
	i = 0;
	entry = -1;
	blk = dip->blocks[0];
	(*buf) = NULL;
	d = NULL;
	/* Search directory entry. */
	while (i < nentries)
	{
		/*
		 * Skip invalid blocks. As directory entries
		 * are removed from a directory, a whole block
		 * may become free.
		 */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/sizeof(struct d_dirent);
			blk = block_map(dip, i*sizeof(struct d_dirent), 0);
			continue;
		}

		/* Get buffer. */
		if ((*buf) == NULL)
		{
			(*buf) = bread(dip->dev, blk);
			d = (*buf)->data;
		}

		/* Get next block */
		else if ((char *)d >= BLOCK_SIZE + (char *) (*buf)->data)
		{
			brelse((*buf));
			(*buf) = NULL;
			blk = block_map(dip, i*sizeof(struct d_dirent), 0);
			continue;
		}

		/* Valid entry. */
		if (d->d_ino != INODE_NULL)
		{
			/* Found */
			if (!kstrncmp(d->d_name, filename, NAME_MAX))
			{
				/* Duplicated entry. */
				if (create)
				{
					brelse((*buf));
					d = NULL;
					curr_proc->errno = EEXIST;
				}

				return (d);
			}
		}

		/* Remember entry index. */
		else
			entry = i;

		d++; i++;
	}

	/* House keeping. */
	if ((*buf) != NULL)
	{
		brelse((*buf));
		(*buf) = NULL;
	}

	/* Create entry. */
	if (create)
	{
		/* Expand directory. */
		if (entry < 0)
		{
			entry = nentries;

			blk = block_map(dip, entry*sizeof(struct d_dirent), 1);

			/* Failed to create entry. */
			if (blk == BLOCK_NULL)
			{
				curr_proc->errno = -ENOSPC;
				return (NULL);
			}

			dip->size += sizeof(struct d_dirent);
			inode_touch(dip);
		}

		else
			blk = block_map(dip, entry*sizeof(struct d_dirent), 0);

		(*buf) = bread(dip->dev, blk);
		entry %= (BLOCK_SIZE/sizeof(struct d_dirent));
		d = &((struct d_dirent *)((*buf)->data))[entry];

		return (d);
	}

	return (NULL);
}

/**
 * @brief Searches for a file in a directory.
 *
 * @details Searches for a file named @p filename in the directory poitned to
 *          by @p dinode.
 *
 * @param ip Inode where the file that shall be searched.
 * @param filename Name of the file that shall be searched.
 *
 * @returns If the requested file exists in the directory, than its inode number
 *          is returned. However, if the file does not exist #INODE_NULL is
 *          is returns instead.
 *
 * @note @p ip must be locked.
 * @note @p filename must point to a valid location.
 */
PUBLIC ino_t dir_search(struct inode *ip, const char *filename)
{
	struct buffer *buf; /* Block buffer.    */
	struct d_dirent *d; /* Directory entry. */

	/* Search directory entry. */
	d = dirent_search(ip, filename, &buf, 0);
	if (d == NULL)
		return (INODE_NULL);

	brelse(buf);

	return (d->d_ino);
}

/*
 * Removes an entry from a directory.
 */
PUBLIC int dir_remove(struct inode *dinode, const char *filename)
{
	struct buffer *buf; /* Block buffer.    */
	struct d_dirent *d; /* Directory entry. */
	struct inode *file; /* File inode.      */

	d = dirent_search(dinode, filename, &buf, 0);

	/* Not found. */
	if (d == NULL)
		return (-ENOENT);

	/* Cannot remove '.' */
	if (d->d_ino == dinode->num)
	{
		brelse(buf);
		return (-EBUSY);
	}

	file = inode_get(dinode->dev, d->d_ino);

	/* Failed to get file's inode. */
	if (file == NULL)
	{
		brelse(buf);
		return (-ENOENT);
	}

	/* Unlinking directory. */
	if (S_ISDIR(file->mode))
	{
		/* Not allowed. */
		if (!IS_SUPERUSER(curr_proc))
		{
			inode_put(file);
			brelse(buf);
			return (-EPERM);
		}

		/* Directory not empty. */
		if (dinode->size)
		{
			inode_put(file);
			brelse(buf);
			return (-EBUSY);
		}
	}

	/* Remove directory entry. */
	d->d_ino = INODE_NULL;
	buf->flags |= BUFFER_DIRTY;
	inode_touch(dinode);
	file->nlinks--;
	inode_touch(file);
	inode_put(file);
	brelse(buf);

	return (0);
}

/*
 * Adds an entry to a directory.
 */
PUBLIC int dir_add(struct inode *dinode, struct inode *inode, const char *name)
{
	struct buffer *buf; /* Block buffer.         */
	struct d_dirent *d; /* Disk directory entry. */

	d = dirent_search(dinode, name, &buf, 1);

	/* Failed to create directory entry. */
	if (d == NULL)
		return (-1);

	kstrncpy(d->d_name, name, NAME_MAX);
	d->d_ino = inode->num;
	buf->flags |= BUFFER_DIRTY;
	brelse(buf);

	return (0);
}

/*
 * Reads from a regular file.
 */
PUBLIC ssize_t file_read(struct inode *i, void *buf, size_t n, off_t off)
{
	char *p;             /* Writing pointer.      */
	size_t blkoff;       /* Block offset.         */
	size_t chunk;        /* Data chunk size.      */
	block_t blk;         /* Working block number. */
	struct buffer *bbuf; /* Working block buffer. */

	p = buf;

	inode_lock(i);

	/* Read data. */
	do
	{
		blk = block_map(i, off, 0);

		/* End of file reached. */
		if (blk == BLOCK_NULL)
			goto out;

		bbuf = bread(i->dev, blk);

		blkoff = off % BLOCK_SIZE;

		/* Calculate read chunk size. */
		chunk = (n < BLOCK_SIZE - blkoff) ? n : BLOCK_SIZE - blkoff;
		if ((off_t)chunk > i->size - off)
		{
			chunk = i->size - off;
			if (chunk == 0)
			{
				brelse(bbuf);
				goto out;
			}
		}

		kmemcpy(p, (char *)bbuf->data + blkoff, chunk);
		brelse(bbuf);

		n -= chunk;
		off += chunk;
		p += chunk;
	} while (n > 0);

out:
	inode_touch(i);
	inode_unlock(i);
	return ((ssize_t)(p - (char *)buf));
}

/*
 * Writes to a regular file.
 */
PUBLIC ssize_t file_write(struct inode *i, const void *buf, size_t n, off_t off)
{
	const char *p;       /* Reading pointer.      */
	size_t blkoff;       /* Block offset.         */
	size_t chunk;        /* Data chunk size.      */
	block_t blk;         /* Working block number. */
	struct buffer *bbuf; /* Working block buffer. */

	p = buf;

	inode_lock(i);

	/* Write data. */
	do
	{
		blk = block_map(i, off, 1);

		/* End of file reached. */
		if (blk == BLOCK_NULL)
			goto out;

		bbuf = bread(i->dev, blk);

		blkoff = off % BLOCK_SIZE;

		chunk = (n < BLOCK_SIZE - blkoff) ? n : BLOCK_SIZE - blkoff;
		kmemcpy((char *)bbuf->data + blkoff, buf, chunk);
		bbuf->flags |= BUFFER_DIRTY;
		brelse(bbuf);

		n -= chunk;
		off += chunk;
		p += chunk;

		/* Update file size. */
		if (off > i->size)
		{
			i->size = off;
			i->flags |= INODE_DIRTY;
		}

	} while (n > 0);

out:

	inode_touch(i);
	inode_unlock(i);
	return ((ssize_t)(p - (char *)buf));
}
