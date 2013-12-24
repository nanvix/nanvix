/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/file.c - File and directories library implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <errno.h>
#include "fs.h"

/*
 * Searchs for a file in a directory.
 */
PUBLIC ino_t dir_search(struct inode *dinode, const char *filename)
{
	int i;              /* Working directory entry index. */
	block_t blk;        /* Working block number.          */
	struct buffer *buf; /* Block buffer.                  */
	struct d_dirent *d; /* Directory entry.               */
	int nentries;       /* Number of directory entries.   */
	
	nentries = dinode->size/sizeof(struct d_dirent);
	
	/* Search from very first block. */
	i = 0;
	blk = dinode->zones[0];		
	buf = NULL;
	
	/* Search directory entry. */
	while (i < nentries)
	{
		/* Get valid block. */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/sizeof(struct d_dirent);
			blk = block_map(dinode, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		/* Get buffer. */
		if (buf == NULL)
		{
			buf = block_read(dinode->dev, blk);
			d = buf->data;
		}
		
		/* Get next block */
		else if ((char *)d >= BLOCK_SIZE + (char *) buf->data)
		{
			block_put(buf);
			buf = NULL;
			blk = block_map(dinode, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		/* Valid entry. */
		if (d->d_ino != INODE_NULL)
		{
			/* Found. */
			if (!kstrncmp(d->d_name, filename, NAME_MAX))
			{
				block_put(buf);
				return (d->d_ino);
			}
		}
		
		d++; i++;	
	}
	
	/* House keeping. */
	if (buf != NULL)
		block_put(buf);
	
	return (INODE_NULL);
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
	struct d_dirent *d; /* Disk directory entry.               */
	int nentries;       /* Actual number of directory entries. */
	
	i = 0;
	entry = -1;
	blk = inode->zones[0];
	buf = NULL;
	
	/* Search for an empty directory entry. */
	while(i < nentries)
	{
		/* Get valid block. */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/sizeof(struct d_dirent);
			blk = block_map(dinode, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		/* Read block buffer. */
		if (buf == NULL)
		{
			buf = block_read(dinode->dev, blk);
			d = buf->data;
		}
		
		/* Get next block. */
		else if ((char *)d >= BLOCK_SIZE + (char *)buf->data)
		{
			block_put(buf);
			buf = NULL;
			blk = block_map(dinode, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		/* Empty entry found. */
		if (d->d_ino == INODE_NULL)
		{
			/* Remember entry index. */
			if (entry < 0)
			{			
				entry = i;
				d->d_ino = inode->num;
				buf->flags |= INODE_DIRTY;
			}
		}
		
		/* Check if entry is not duplicated. */
		else
		{
			/* Found duplicated entry. */
			if (!kstrncmp(d->d_name, name, NAME_MAX))
			{
				block_put(buf);
				
				/* Free remembered entry index. */
				if (entry >= 0)
				{
					blk = block_map(dinode, entry*sizeof(struct d_dirent), 0);
					buf = block_read(dinode->dev, blk);
					entry %= (BLOCK_SIZE/sizeof(struct d_dirent));
					d = &((struct d_dirent *)(buf->data))[entry];
					d->d_ino = 0;
					buf->flags |= BUFFER_DIRTY;
					block_put(buf);
				}
				
				return (-1);
			}
		}
		
		d++; i++;
	}
	
	/* House keeping. */
	if (buf != NULL)
		block_put(buf);
	
	/* Create entry. */
	if (entry < 0)
	{
		entry = nentries;
		
		blk = block_map(dinode, entry*sizeof(struct d_dirent), 1);
		
		/* Failed to create entry. */
		if (blk == BLOCK_NULL)
		{
			curr_proc->errno = -ENOSPC;
			return (-1);
		}
	}
	
	else
		blk = block_map(dinode, entry*sizeof(struct d_dirent), 0);
	
	buf = block_read(dinode->dev, blk);
	entry %= (BLOCK_SIZE/sizeof(struct d_dirent));
	d = &((struct d_dirent *)(buf->data))[entry];
	kstrncpy(d->d_name, name, NAME_MAX);
	buf->flags |= INODE_DIRTY;
	block_put(buf);
	return (entry);
}

/*
 * Reads from a regular file.
 */
PUBLIC ssize_t file_read(struct inode *i, void *buf, size_t n, off_t off)
{
	char *p;             /* Writing pointer.      */
	size_t chunk;        /* Data chunk size.      */
	block_t blk;         /* Working block number. */
	struct buffer *bbuf; /* Working block buffer. */
		
	p = buf;
	
	/* Read data. */
	do
	{
		blk = block_map(i, off, 0);
		
		/* End of file reached. */
		if (blk == BLOCK_NULL)
			return ((ssize_t)(p - (char *)buf));
		
		bbuf = block_read(i->dev, blk);
		
		chunk = (n < BLOCK_SIZE) ? n : BLOCK_SIZE;
		kmemcpy(p, bbuf->data, chunk);
		block_put(bbuf);
		
		n -= chunk;
		off += chunk;
		p += chunk;
	} while (n > 0);
	
	return ((ssize_t)(p - (char *)buf));
}

/*
 * Writes to a regular file.
 */
PUBLIC ssize_t file_write(struct inode *i, const void *buf, size_t n, off_t off)
{
	const char *p;       /* Reading pointer.      */
	size_t chunk;        /* Data chunk size.      */
	block_t blk;         /* Working block number. */
	struct buffer *bbuf; /* Working block buffer. */
		
	p = buf;
	
	/* Write data. */
	do
	{
		blk = block_map(i, off, 1);
		
		/* End of file reached. */
		if (blk == BLOCK_NULL)
			return ((ssize_t)(p - (char *)buf));
		
		bbuf = block_read(i->dev, blk);
		chunk = (n < BLOCK_SIZE) ? n : BLOCK_SIZE;
		kmemcpy(bbuf->data, buf, chunk);
		bbuf->flags |= BUFFER_DIRTY;
		block_put(bbuf);
		
		n -= chunk;
		off += chunk;
		p += chunk;
	} while (n > 0);
	
	return ((ssize_t)(p - (char *)buf));
}
