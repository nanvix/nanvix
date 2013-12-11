/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/inode.c - Inode library implementation.
 */

#include <nanvix/clock.h>
#include <nanvix/config.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
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
	i->flags = (i->flags & ~(INODE_DIRTY | INODE_MOUNT)) | INODE_VALID;
	
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
 * Locks an inode.
 */
PUBLIC void inode_lock(struct inode *i)
{
	/* Wait for inode to become unlocked. */
	while (i->flags & INODE_LOCKED)
		sleep(&i->chain, PRIO_INODE);
		
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
			inode_write(i);
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
		i->zones[j] = NO_ZONE;
	}
	for (j = 0; j < NR_ZONES_SINGLE; j++)
	{
		zone_free_indirect(sb, i->zones[NR_ZONES_DIRECT + j]);
		i->zones[NR_ZONES_DIRECT + j] = NO_ZONE;
	}
	for (j = 0; j < NR_ZONES_DOUBLE; j++)
	{
		zone_free_indirect(sb, i->zones[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j]);
		i->zones[NR_ZONES_DIRECT + NR_ZONES_SINGLE + j] = NO_ZONE;
	}
	
	superblock_unlock(sb);
	
	i->size = 0;
	i->flags |= INODE_DIRTY;
	i->flags |= CURRENT_TIME;
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
	
	goto error0;
	
found:

	i = inode_cache_evict();
	
	/* No free inode. */
	if (i == NULL)
		goto error0;
	
	num = bit + blk*(BLOCK_SIZE << 3);
	
	/* Allocate inode. */
	bitmap_set(sb->imap[blk]->data, bit);
	sb->imap[blk]->flags |= BUFFER_DIRTY;
	sb->isearch = num;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	/* Initialize inode. */
	i->nlinks = 1;
	i->uid = curr_proc->uid;
	i->gid = curr_proc->gid;
	i->size = 0;
	i->time = CURRENT_TIME;
	for (j = 0; j < NR_ZONES; j++)
		i->zones[j] = 0;
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
 * Releases access to inode.
 */
PUBLIC void inode_put(struct inode *i)
{
	/* Release in-core inode. */
	if (--i->count == 0)
	{		
		/* Free underlying disk blocks. */
		if (i->nlinks == 0)
		{
			inode_free(i);
			inode_truncate(i);
		}
		
		inode_write(i);
		
		/* Insert inode in the free list. */
		i->free_next = free_inodes;
		free_inodes = i;
		
		inode_cache_remove(i);
		
		i->flags &= ~INODE_VALID;
	}
	
	inode_unlock(i);
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
		inodes[i].flags = ~(INODE_DIRTY|INODE_VALID|INODE_LOCKED|INODE_MOUNT);
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
