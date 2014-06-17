/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/zone.c - Zone library implementation.
 */

#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <errno.h>
#include "fs.h"

/*
 * Allocates a zone.
 */
PUBLIC zone_t zone_alloc(struct superblock *sb)
{
	bit_t bit;          /* Bit in the zone bitmap.  */
	zone_t num;         /* Zone number.             */
	block_t blk;        /* Current block on search. */
	struct buffer *buf; /* Working buffer.          */

	blk = sb->zsearch/(BLOCK_SIZE << 3);
	
	/* Search for free zone. */
	do
	{
		bit = bitmap_first_free(sb->zmap[blk]->data, BLOCK_SIZE);
		
		/* Found. */
		if (bit != BITMAP_FULL)
			goto found;
		
		blk++;
		if (blk < sb->zmap_blocks)
			blk = 0;
		
	} while (blk != sb->zsearch/(BLOCK_SIZE << 3));
	
	return (ZONE_NULL);

found:

	num = bit + blk*(BLOCK_SIZE << 3);
	
	/* Allocate zone. */
	bitmap_set(sb->zmap[blk]->data, bit);
	sb->zmap[blk]->flags |= BUFFER_DIRTY;
	sb->zsearch = num;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	/*
	 * Clean underlying blocks in order
	 * to avoid security issues.
	 */
	for (blk = num; blk < num + (1 << sb->log_zone_size); blk++)
	{
		buf = bread(sb->dev, blk);
		kmemset(buf->data, 0, BLOCK_SIZE);
		buf->flags |= BUFFER_DIRTY;
		brelse(buf);
	}
	
	return (num);
}

/*
 * Frees a zone.
 */
PUBLIC void zone_free(struct superblock *sb, zone_t num)
{
	block_t blk;
	
	/* Nothing to be done. */
	if (num == ZONE_NULL)
		return;
	
	blk = num/(BLOCK_SIZE << 3);
	
	/* Free zone. */
	bitmap_clear(sb->zmap[blk]->data, num%(BLOCK_SIZE << 3));
	sb->zmap[blk]->flags |= BUFFER_DIRTY;
	if (num < sb->zsearch)
		sb->zsearch = num;
	sb->flags |= SUPERBLOCK_DIRTY;
}

/*
 * Frees an indirect zone.
 */
PUBLIC void zone_free_indirect(struct superblock *sb, zone_t num)
{
	int i, j;           /* Loop indexes.     */
	int nzones;         /* Number of zones.  */
	int nblocks;        /* Number of blocks. */
	struct buffer *buf; /* Block buffer.     */
	
	/* Nothing to be done. */
	if (num == ZONE_NULL)
		return;
	
	nblocks = (BLOCK_SIZE << sb->log_zone_size)/BLOCK_SIZE;
	nzones = BLOCK_SIZE/sizeof(zone_t);
	
	/* Free indirect zone. */
	for (i = 0; i < nblocks; i++)
	{
		buf = bread(sb->dev, num + i);
		
		/* Free direct zone. */
		for (j = 0; j < nzones; j++)
			zone_free(sb, ((zone_t *)buf->data)[j]);
		
		brelse(buf);
	}
	
	zone_free(sb, num);
}

/*
 * Frees a doubly indirect zone.
 */
PUBLIC void zone_free_dindirect(struct superblock *sb, zone_t num)
{
	int i, j;           /* Loop indexes.     */
	int nzones;         /* Number of zones.  */
	int nblocks;        /* Number of blocks. */
	zone_t zone;        /* Zone.             */
	struct buffer *buf; /* Block buffer.     */
	
	/* Nothing to be done. */
	if (num == ZONE_NULL)
		return;
	
	nblocks = (BLOCK_SIZE << sb->log_zone_size)/BLOCK_SIZE;
	nzones = BLOCK_SIZE/sizeof(zone_t);
	
	/* Free indirect zone. */
	for (i = 0; i < nblocks; i++)
	{
		buf = bread(sb->dev, num + i);
		
		/* Free direct zone. */
		for (j = 0; j < nzones; j++)
		{
			if ((zone = ((zone_t *)buf->data)[j]))
				zone_free_indirect(sb, zone);
		}
		
		brelse(buf);
	}
	
	zone_free(sb, num);
}

/*
 * Maps a file byte offset in a physical zone number.
 */
PUBLIC zone_t zone_map(struct inode *inode, off_t off, int create)
{
	zone_t phys;        /* Physical zone number. */
	zone_t logic;       /* Logical zone number.  */
	struct buffer *buf; /* Underlying buffer.    */
	
	logic = off/BLOCK_SIZE;
	
	/* Offset too big. */
	if (off/BLOCK_SIZE >= (ssize_t)(NR_DIRECT + NR_SINGLE + NR_DOUBLE))
	{
		curr_proc->errno = -EFBIG;
		return (BLOCK_NULL);
	}
	
	/* 
	 * Create blocks that are in a
	 * valid offset.
	 */
	if (off < inode->size)
		create = 1;
	
	/* Direct zone. */
	if (logic < NR_ZONES_DIRECT)
	{
		/* Create zone. */
		if (inode->zones[logic] == ZONE_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = zone_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != ZONE_NULL)
			{	
				inode->zones[logic] = phys;		
				inode->time = CURRENT_TIME;
				inode->flags |= INODE_DIRTY;
			}
		}
		
		return (inode->zones[logic]);
	}
	
	logic -= NR_ZONES_DIRECT;
	
	/* Single indirect zone. */
	if (logic < NR_SINGLE)
	{		
		/* Create zone. */
		if (inode->zones[ZONE_SINGLE] == ZONE_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = zone_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys == ZONE_NULL)
				return (ZONE_NULL);
				
			inode->zones[ZONE_SINGLE] = phys;		
			inode->time = CURRENT_TIME;
			inode->flags |= INODE_DIRTY;
		}
		
		if ((phys = inode->zones[ZONE_SINGLE]) == ZONE_NULL)
			return (ZONE_NULL);
	
		buf = bread(inode->dev, phys);
		
		if ((((zone_t *)buf->data)[logic] == ZONE_NULL) && create)
		{
			superblock_lock(inode->sb);
			phys = zone_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != ZONE_NULL)
			{	
				((zone_t *)buf->data)[logic] = phys;		
				inode->time = CURRENT_TIME;
				inode->flags |= INODE_DIRTY;
			}
		}
		
		brelse(buf);
		
		return (((zone_t *)buf->data)[logic]);
	}
	
	logic -= NR_SINGLE;
	
	/* Double indirect zone. */
	kpanic("double indirect zones not supported yet");
	
	return (ZONE_NULL);
}
