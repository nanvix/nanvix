/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/inode.c - Inode library implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/paging.h>
#include <errno.h>
#include <unistd.h>
#include <limits.h>
#include "fs.h"

/* Number of inodes per block. */
#define INODES_PER_BLOCK (BLOCK_SIZE/sizeof(struct d_inode))

/* Hash table size. */
#define HASHTAB_SIZE 227

/* In-core inodes. */
PRIVATE struct inode inodes[NR_INODES];

/* Free inodes. */
PRIVATE struct inode *free_inodes = NULL;

/* Inodes hash table. */
PRIVATE struct inode *hashtab[HASHTAB_SIZE];

/*
 * Hash function.
 */
#define HASH(dev, num) \
	(((dev)^(num))%HASHTAB_SIZE)

/*
 * Removes an inode from the free list.
 */
PRIVATE struct inode *inode_cache_evict(void)
{
	struct inode *i;
	
	/* No free inodes. */
	if (free_inodes == NULL)
	{
		kprintf("inode table overflow");
		return (NULL);
	}
	
	/* Remove inode from free list. */
	i = free_inodes;
	free_inodes = free_inodes->free_next;
	
	i->count++;
	inode_lock(i);
	
	return (i);
}

/*
 * Inserts an inode in the cache.
 */
PRIVATE void inode_cache_insert(struct inode *i)
{
	/* Insert inode in the hash table. */
	i->hash_next = hashtab[HASH(i->dev, i->num)];
	i->hash_prev = NULL;
	hashtab[HASH(i->dev, i->num)] = i;
	if (i->hash_next != NULL)
		i->hash_next->hash_prev = i;
}

/*
 * Removes an inode from the cache.
 */
PRIVATE void inode_cache_remove(struct inode *i)
{
	/* Remove inode from the hash table. */
	if (i->hash_prev != NULL)
		i->hash_prev->hash_next = i->hash_next;
	if (i->hash_next != NULL)
		i->hash_next->hash_prev = i->hash_prev;
	if (hashtab[HASH(i->dev, i->num)] == i)
		hashtab[HASH(i->dev, i->num)] = i->hash_next;
}

/*
 * Writes an inode to disk.
 */
PRIVATE void inode_write(struct inode *i)
{
	int j;                 /* Loop index.  */
	unsigned blk;          /* Block.       */
	struct buffer *buf;    /* Buffer.      */
	struct d_inode *d_i;   /* Disk inode.  */
	struct superblock *sb; /* Super block. */
	
	/* Nothing to be done. */
	if (!(i->flags & INODE_DIRTY))
		return;
	
	superblock_lock(sb = i->sb);
	
	blk = 2 + sb->imap_blocks + sb->zmap_blocks + i->num/INODES_PER_BLOCK;
	
	buf = block_read(i->dev, blk);
	
	/* Failed to read inode. */
	if (buf == NULL)
		kpanic("failed to write inode");
	
	d_i = &(((struct d_inode *)buf->data)[i->num%INODES_PER_BLOCK - 1]);
	
	d_i->i_mode = i->mode;
	d_i->i_nlinks = i->nlinks;
	d_i->i_uid = i->uid;
	d_i->i_gid = i->gid;
	d_i->i_size = i->size;
	d_i->i_time = i->time;
	for (j = 0; j < NR_ZONES; j++)
		d_i->i_zones[j] = i->zones[j];
	i->flags &= ~INODE_DIRTY;
	buf->flags |= BUFFER_DIRTY;
	
	block_put(buf);
	superblock_unlock(sb);
}

/*
 * Reads an inode from disk.
 */
PRIVATE struct inode *inode_read(dev_t dev, ino_t num)
{
	int j;                 /* Loop index.    */
	unsigned blk;          /* Block.         */
	struct inode *i;       /* In-core inode. */
	struct buffer *buf;    /* Buffer.        */
	struct d_inode *d_i;   /* Disk inode.    */
	struct superblock *sb; /* Super block.   */
	
	sb = superblock_get(dev);
	
	/* Invalid device. */
	if (sb == NULL)
		goto error0;	
	
	/* Calculate block number. */
	blk = 2 + sb->imap_blocks + sb->zmap_blocks + num/INODES_PER_BLOCK;
	
	buf = block_read(dev, blk);
	
	/* Failed to read inode. */
	if (buf == NULL)
		kpanic("failed to read inode");
	
	d_i = &(((struct d_inode *)buf->data)[num%INODES_PER_BLOCK - 1]);
	
	/* Invalid inode. */
	if (d_i->i_mode == 0)
		goto error0;
	
	i = inode_cache_evict();
	
	/* No free inode. */
	if (i == NULL)
		goto error1;
		
	/* Initialize in-core inode. */
	i->mode = d_i->i_mode;
	i->nlinks = d_i->i_nlinks;
	i->uid = d_i->i_uid;
	i->gid = d_i->i_gid;
	i->size = d_i->i_size;
	i->time = d_i->i_time;
	for (j = 0; j < NR_ZONES; j++)
		i->zones[j] = d_i->i_zones[j];
	i->dev = dev;
	i->num = num;
	i->sb = sb;
	i->count = 1;
	i->flags = (INODE_DIRTY | INODE_MOUNT) & INODE_VALID;
	
	block_put(buf);
	superblock_unlock(sb);
	
	return (i);

error1:
	superblock_unlock(sb);
error0:
	return (NULL);
}

/*
 * Frees an inode.
 */
PRIVATE void inode_free(struct inode *i)
{
	block_t blk;           /* Block number.           */
	struct superblock *sb; /* Underlying super block. */
	
	blk = i->num/(BLOCK_SIZE << 3);
	
	superblock_lock(sb = i->sb);
	
	bitmap_clear(sb->imap[blk], i->num%(BLOCK_SIZE << 3));
	sb->imap[blk]->flags |= BUFFER_DIRTY;
	if (i->num < sb->zsearch)
		sb->zsearch = i->num;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	superblock_unlock(sb);
	
	i->mode = 0;
	i->flags |= INODE_DIRTY;
}

/*
 * Waits for an inode to become unlocked.
 */
PRIVATE void inode_wait(struct inode *i)
{
	while (i->flags & INODE_LOCKED)
		sleep(&i->chain, PRIO_INODE);
}

/*
 * Locks an inode.
 */
PUBLIC void inode_lock(struct inode *i)
{
	inode_wait(i);
	i->flags |= INODE_LOCKED;
}

/*
 * Unlocks an inode.
 */
PUBLIC void inode_unlock(struct inode *i)
{
	wakeup(&i->chain);
	i->flags &= ~INODE_LOCKED;
}

/*
 * Synchronizes all inodes.
 */
PUBLIC void inode_sync(void)
{
	struct inode *i;
	
	/* Write valid inodes to disk. */
	for (i = &inodes[0]; i < &inodes[NR_INODES]; i++)
	{
		if (i->flags & INODE_VALID)
		{
			inode_wait(i);
			if (!(i->flags & INODE_PIPE))
				inode_write(i);
		}
	}
}

/*
 * Truncates an inode.
 */
PUBLIC void inode_truncate(struct inode *i)
{
	int j;                 /* Loop index.             */
	struct superblock *sb; /* Underlying super block. */
	
	superblock_lock(sb = i->sb);
	
	/* Free file zones. */
	for (j = 0; j < NR_ZONES_DIRECT; j++)
	{
		zone_free(sb, i->zones[j]);
		i->zones[j] = ZONE_NULL;
	}
	for (j = 0; j < NR_ZONES_SINGLE; j++)
	{
		zone_free_indirect(sb, i->zones[NR_ZONES_DIRECT + j]);
		i->zones[NR_ZONES_DIRECT + j] = ZONE_NULL;
	}
	for (j = 0; j < NR_ZONES_DOUBLE; j++)
	{
		zone_free_indirect(sb, i->zones[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j]);
		i->zones[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j] = ZONE_NULL;
	}
	
	superblock_unlock(sb);
	
	i->size = 0;
	i->time = CURRENT_TIME;
	i->flags |= INODE_DIRTY;
}

/*
 * Allocates an inode.
 */
PUBLIC struct inode *inode_alloc(struct superblock *sb)
{
	int j;              /* Loop index.               */
	ino_t num;          /* Inode number.             */
	bit_t bit;          /* Bit number in the bitmap. */
	block_t blk;        /* Number of current block.  */
	struct inode *i;    /* Inode.                    */
	
	superblock_lock(sb);
	
	blk = sb->isearch/(BLOCK_SIZE << 3);
	
	/* Search for free inode. */
	do
	{			
		bit = bitmap_first_free(sb->imap[blk]->data, BLOCK_SIZE);
	
		/* Found. */
		if (bit != BITMAP_FULL)
			goto found;
		
		blk++;
		if (blk < sb->imap_blocks)
			blk = 0;
		
	} while (blk != sb->isearch/(BLOCK_SIZE << 3));
	
	curr_proc->errno = -ENOSPC;
	goto error0;
	
found:

	i = inode_cache_evict();
	
	/* No free inode. */
	if (i == NULL)
	{
		curr_proc->errno = -ENFILE;
		goto error0;
	}
	
	num = bit + blk*(BLOCK_SIZE << 3);
	
	/* Allocate inode. */
	bitmap_set(sb->imap[blk]->data, bit);
	sb->imap[blk]->flags |= BUFFER_DIRTY;
	sb->isearch = num;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	/* 
	 * Initialize inode. 
	 * mode_t will be initialized later.
	 */
	i->nlinks = 1;
	i->uid = curr_proc->euid;
	i->gid = curr_proc->egid;
	i->size = 0;
	i->time = CURRENT_TIME;
	for (j = 0; j < NR_ZONES; j++)
		i->zones[j] = ZONE_NULL;
	i->dev = sb->dev;
	i->num = num;
	i->sb = sb;
	i->count = 1;
	i->flags |= INODE_DIRTY;
	i->chain = NULL;
	
	inode_cache_insert(i);
	
	superblock_unlock(sb);
	
	return (i);
	
error0:
	superblock_unlock(sb);
	return (NULL);
}

/*
 * Gets access to inode.
 */
PUBLIC struct inode *inode_get(dev_t dev, ino_t num)
{
	struct inode *i;

repeat:

	/* Search in the hash table. */
	for (i = hashtab[HASH(dev, num)]; i != NULL; i = i->hash_next)
	{
		/* No found. */
		if (i->dev != dev || i->num != num)
			continue;
					
		/* Inode is locked. */
		if (i->flags & INODE_LOCKED)
		{
			sleep(&i->chain, PRIO_INODE);
			goto repeat;
		}
		
		/* Cross mount point. */
		if (i->flags & INODE_MOUNT)
		{
			 dev = i->dev;
			 goto repeat;
		}
		
		i->count++;
		inode_lock(i);
		
		return (i);
	}
	
	i = inode_read(dev, num);
	
	/* No free inode. */
	if (i == NULL)
		return (NULL);
	
	inode_cache_insert(i);
	
	return (i);
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
	inode->flags = ~(INODE_DIRTY | INODE_MOUNT) & (INODE_VALID | INODE_PIPE);
	inode->pipe = pipe;
	inode->head = 0;
	inode->tail = 0;
	
	return (inode);

error1:
	putkpg(pipe);
error0:
	return (NULL);
}

/*
 * Releases access to inode.
 */
PUBLIC void inode_put(struct inode *i)
{
	/* Release in-core inode. */
	if (--i->count == 0)
	{
		if (i->flags & INODE_PIPE)
			putkpg(i->pipe);	
		else
		{		
			/* Free underlying disk blocks. */
			if (i->nlinks == 0)
			{
				inode_free(i);
				inode_truncate(i);
			}
			
			inode_write(i);
			
			inode_cache_remove(i);
		}
		
		
		/* Insert inode in the free list. */
		i->free_next = free_inodes;
		free_inodes = i;
		
		i->flags &= ~INODE_VALID;
	}
	
	inode_unlock(i);
}

/*
 * Breaks a path.
 */
PRIVATE const char *break_path(const char *pathname, char *filename)
{
	char *p2;
	const char *p1;
	
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
		{
			curr_proc->errno = -ENAMETOOLONG;
			return (NULL);
		}
		
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

/*
 * Initializes inodes.
 */
PUBLIC void inode_init(void)
{
	int i;
	
	kprintf("fs: initializing inode cache");
	
	/* Initialize inodes. */
	for (i = 0; i < NR_INODES; i++)
	{
		inodes[i].count = 0;
		inodes[i].flags = ~(INODE_VALID | INODE_LOCKED);
		inodes[i].chain = NULL;
		inodes[i].free_next = &inodes[(i + 1)%NR_INODES];
		inodes[i].hash_next = NULL;
		inodes[i].hash_prev = NULL;
	}
	
	/* Initialize inode cache. */
	free_inodes = &inodes[0];
	inodes[NR_INODES - 1].free_next = NULL;
	for (i = 0; i < HASHTAB_SIZE; i++)
		hashtab[i] = NULL;
}
