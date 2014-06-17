/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/file.c - File and directories library implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include "fs.h"

/*
 * Searches for a directory entry.
 */
PRIVATE struct dirent *dirent_search
(struct inode *dinode, const char *filename, struct buffer **buf, int *entry)
{
	int i;            /* Working directory entry index. */
	block_t blk;      /* Working block number.          */
	struct dirent *d; /* Directory entry.               */
	int nentries;     /* Number of directory entries.   */
	
	nentries = dinode->size/_SIZEOF_DIRENT;
	
	/* Search from very first block. */
	i = 0;
	*entry = -1;
	blk = dinode->blocks[0];		
	(*buf) = NULL;
	
	/* Search directory entry. */
	while (i < nentries)
	{
		/* Get valid block. */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/_SIZEOF_DIRENT;
			blk = block_map(dinode, i*_SIZEOF_DIRENT, 0);
			continue;
		}
		
		/* Get buffer. */
		if ((*buf) == NULL)
		{
			(*buf) = bread(dinode->dev, blk);
			d = (*buf)->data;
		}
		
		/* Get next block */
		else if ((char *)d >= BLOCK_SIZE + (char *) (*buf)->data)
		{
			brelse((*buf));
			(*buf) = NULL;
			blk = block_map(dinode, i*_SIZEOF_DIRENT, 0);
			continue;
		}
		
		/* Valid entry. */
		if (d->d_ino != INODE_NULL)
		{
			/* Found */
			if (!kstrncmp(d->d_name, filename, NAME_MAX))
			{
				/* Duplicated entry. */
				if (entry != NULL)
				{
					*entry = -1;
					brelse((*buf));
					d = NULL;
				}
				
				return (d);
			}
		}

		/* Creating entry. */
		else if (entry != NULL)
		{
			/* Remember entry index. */
			if (*entry < 0)
				*entry = i;
		}
		
		d++; i++;	
	}
	
	/* Creating entry. */
	if (entry != NULL)
		*entry = nentries;
	
	/* House keeping. */
	if ((*buf) != NULL)
	{
		brelse((*buf));
		(*buf) = NULL;
	}
	
	return (NULL);
}

/*
 * Searchs for a file in a directory.
 */
PUBLIC ino_t dir_search(struct inode *dinode, const char *filename)
{
	struct buffer *buf; /* Block buffer.    */
	struct dirent *d;   /* Directory entry. */
	
	d = dirent_search(dinode, filename, &buf, NULL);
	
	/* Not found. */
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
	struct dirent *d;   /* Directory entry. */
	struct inode *file; /* File inode.      */
	
	d = dirent_search(dinode, filename, &buf, NULL);
	
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
	d->d_ino = 0;
	buf->flags |= BUFFER_DIRTY;
	dinode->time = CURRENT_TIME;
	dinode->flags |= INODE_DIRTY;
	file->nlinks--;
	file->time = CURRENT_TIME;
	file->flags |= INODE_DIRTY;
	inode_put(file);
	brelse(buf);
	inode_put(dinode);
	
	return (0);
}

/*
 * Adds an entry to a directory.
 */
PUBLIC int dir_add(struct inode *dinode, struct inode *inode, const char *name)
{
	int i;              /* Working directory entry index.      */
	int entry;          /* Inode entry index.                  */
	block_t blk;        /* Working block number.               */
	struct buffer *buf; /* Block buffer.                       */
	struct dirent *d;   /* Disk directory entry.               */
	int nentries;       /* Actual number of directory entries. */
	
	nentries = dinode->size/_SIZEOF_DIRENT;
	
	i = 0;
	entry = -1;
	blk = inode->blocks[0];
	buf = NULL;
	
	/* Search for an empty directory entry. */
	while(i < nentries)
	{
		/* Get valid block. */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/_SIZEOF_DIRENT;
			blk = block_map(dinode, i*_SIZEOF_DIRENT, 0);
			continue;
		}
		
		/* Read block buffer. */
		if (buf == NULL)
		{
			buf = bread(dinode->dev, blk);
			d = buf->data;
		}
		
		/* Get next block. */
		else if ((char *)d >= BLOCK_SIZE + (char *)buf->data)
		{
			brelse(buf);
			buf = NULL;
			blk = block_map(dinode, i*_SIZEOF_DIRENT, 0);
			continue;
		}
		
		/* Empty entry found. */
		if (d->d_ino == INODE_NULL)
		{
			/* Remember entry index. */
			if (entry < 0)
				entry = i;
		}
		
		/* Check if entry is not duplicated. */
		else
		{
			/* Found duplicated entry. */
			if (!kstrncmp(d->d_name, name, NAME_MAX))
			{
				brelse(buf);
				
				return (-1);
			}
		}
		
		d++; i++;
	}
	
	/* House keeping. */
	if (buf != NULL)
		brelse(buf);
	
	/* Create entry. */
	if (entry < 0)
	{
		entry = nentries;
		
		blk = block_map(dinode, entry*_SIZEOF_DIRENT, 1);
		
		/* Failed to create entry. */
		if (blk == BLOCK_NULL)
		{
			curr_proc->errno = -ENOSPC;
			return (-1);
		}
		
		dinode->size += sizeof(struct dirent);
		dinode->flags |= INODE_DIRTY;
	}
	
	else
		blk = block_map(dinode, entry*_SIZEOF_DIRENT, 0);
	
	buf = bread(dinode->dev, blk);
	entry %= (BLOCK_SIZE/_SIZEOF_DIRENT);
	d = &((struct dirent *)(buf->data))[entry];
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
		
		/* Failed to read. */
		if (bbuf == NULL)
			return (-1);
			
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
	i->time = CURRENT_TIME;
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
		
		/* Failed to read block. */
		if (bbuf == NULL)
			return (-1);
		
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

	i->time = CURRENT_TIME;
	inode_unlock(i);
	return ((ssize_t)(p - (char *)buf));
}
