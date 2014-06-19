/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/zone.c - Zone library implementation.
 */

#include <nanvix/bitmap.h>
#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <errno.h>
#include "fs.h"

/*
 * Allocates a zone.
 */
PUBLIC block_t block_alloc(struct superblock *sb)
{
	bit_t bit;          /* Bit in the zone bitmap.  */
	block_t num;         /* Zone number.             */
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
	
	return (BLOCK_NULL);

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
	for (blk = num; blk < num + 1; blk++)
	{
		buf = bread(sb->dev, blk);
		kmemset(buf->data, 0, BLOCK_SIZE);
		buf->flags |= BUFFER_DIRTY;
		brelse(buf);
	}
	
	return (num);
}

/*
 * Frees a direct disk block.
 */
PRIVATE void block_free_direct(struct superblock *sb, block_t num)
{
	int idx; /* Bitmap index.  */
	int off; /* Bitmap offset. */
	
	/* Nothing to be done. */
	if (num == BLOCK_NULL)
		return;
		
	/* 
	 * Remember free disk to
	 * speedup next block allocation.
	 */
	if (num < sb->zsearch)
		sb->zsearch = num;
	
	/*
	 * Compute block index and offset 
	 * in the bitmap.
	 */
	idx = num/(BLOCK_SIZE << 3);
	off = num%(BLOCK_SIZE << 3);
	
	/* Free disk block. */
	bitmap_clear(sb->zmap[idx]->data, off);
	sb->zmap[idx]->flags |= BUFFER_DIRTY;
	sb->flags |= SUPERBLOCK_DIRTY;
}

/*
 * Frees an indirect disk block.
 */
PRIVATE void block_free_indirect(struct superblock *sb, block_t num)
{
	int i;              /* Loop index.   */
	struct buffer *buf; /* Block buffer. */
	
	/* Nothing to be done. */
	if (num == BLOCK_NULL)
		return;
	
	buf = bread(sb->dev, num);
		
	/* Free indirect disk block. */
	for (i = 0; i < NR_SINGLE; i++)
		block_free_direct(sb, ((block_t *)buf->data)[i]);
	block_free_direct(sb, num);
		
	brelse(buf);
}

/*
 * Frees a doubly indirect zone.
 */
PRIVATE void block_free_dindirect(struct superblock *sb, block_t num)
{
	int i;              /* Loop indexes. */
	block_t zone;       /* Zone.         */
	struct buffer *buf; /* Block buffer. */
	
	/* Nothing to be done. */
	if (num == BLOCK_NULL)
		return;
	
	buf = bread(sb->dev, num);
		
	/* Free direct zone. */
	for (i = 0; i < NR_SINGLE; i++)
	{
		if ((zone = ((block_t *)buf->data)[i]))
			block_free_indirect(sb, zone);
	}
		
	brelse(buf);
	
	block_free_direct(sb, num);
}

/*
 * Frees a disk block.
 */
PUBLIC void block_free(struct superblock *sb, block_t num, int lvl)
{
	/* Free disk block. */
	switch (lvl)
	{
		/* Direct block. */
		case 0:
			block_free_direct(sb, num);
			break;
		
		/* Single indirect block. */
		case 1:
			block_free_indirect(sb, num);
			break;
		
		/* Doubly indirect block. */
		case 2:
			block_free_dindirect(sb, num);
			break;
		
		/* Should not happen. */
		default:
			kpanic("block_free() bad indirection level");
	}
}

/*
 * Maps a file byte offset in a physical zone number.
 */
PUBLIC block_t block_map(struct inode *inode, off_t off, int create)
{
	block_t phys;        /* Physical zone number. */
	block_t logic;       /* Logical zone number.  */
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
		if (inode->blocks[logic] == BLOCK_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != BLOCK_NULL)
			{	
				inode->blocks[logic] = phys;		
				inode->time = CURRENT_TIME;
				inode->flags |= INODE_DIRTY;
			}
		}
		
		return (inode->blocks[logic]);
	}
	
	logic -= NR_ZONES_DIRECT;
	
	/* Single indirect zone. */
	if (logic < NR_SINGLE)
	{		
		/* Create zone. */
		if (inode->blocks[ZONE_SINGLE] == BLOCK_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys == BLOCK_NULL)
				return (BLOCK_NULL);
				
			inode->blocks[ZONE_SINGLE] = phys;		
			inode->time = CURRENT_TIME;
			inode->flags |= INODE_DIRTY;
		}
		
		if ((phys = inode->blocks[ZONE_SINGLE]) == BLOCK_NULL)
			return (BLOCK_NULL);
	
		buf = bread(inode->dev, phys);
		
		if ((((block_t *)buf->data)[logic] == BLOCK_NULL) && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != BLOCK_NULL)
			{	
				((block_t *)buf->data)[logic] = phys;		
				inode->time = CURRENT_TIME;
				inode->flags |= INODE_DIRTY;
			}
		}
		
		brelse(buf);
		
		return (((block_t *)buf->data)[logic]);
	}
	
	logic -= NR_SINGLE;
	
	/* Double indirect zone. */
	kpanic("double indirect zones not supported yet");
	
	return (BLOCK_NULL);
}
