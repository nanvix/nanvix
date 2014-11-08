/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/block.c - Block library implementation.
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

#include <nanvix/bitmap.h>
#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <errno.h>

/**
 * @brief Allocates a disk block.
 * 
 * @details Allocates a disk block by searching in the bitmap of blocks for a
 *          free block.
 * 
 * @param sb Superblock in which the disk block should be allocated.
 * 
 * @return Upon successful completion, the block number of the allocated block
 *         is returned. Upon failed, #BLOCK_NULL is returned instead.
 * 
 * @note The superblock must be locked.
 */
PRIVATE block_t block_alloc(struct superblock *sb)
{
	bit_t bit;          /* Bit number in the bitmap. */
	block_t num;        /* Block number.             */
	block_t blk;        /* Working block.            */
	block_t firstblk;   /* First block to check.     */
	struct buffer *buf; /* Working buffer.           */

	/* Search for a free block. */
	firstblk = (sb->zsearch - sb->first_data_block)/(BLOCK_SIZE << 3);
	blk = firstblk;
	do
	{
		bit = bitmap_first_free(sb->zmap[blk]->data, BLOCK_SIZE);
		
		/* Found. */
		if (bit != BITMAP_FULL)
			goto found;
		
		/* Wrap around. */
		blk = (blk + 1 < sb->zmap_blocks) ? blk + 1 : 0;
	} while (blk != firstblk);
	
	return (BLOCK_NULL);

found:

	num =  sb->first_data_block + bit + blk*(BLOCK_SIZE << 3);
	
	/* 
	 * Remember disk block number to 
	 * speedup next block allocation.
	 */
	sb->zsearch = num;
	
	/* Allocate block. */
	bitmap_set(sb->zmap[blk]->data, bit);
	sb->zmap[blk]->flags |= BUFFER_DIRTY;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	/* Clean block to avoid security issues. */
	buf = bread(sb->dev, blk);	
	kmemset(buf->data, 0, BLOCK_SIZE);
	buf->flags |= BUFFER_DIRTY;
	brelse(buf);
	
	return (num);
}

/**
 * @brief Frees a direct disk block.
 * 
 * @details Frees a direct disk block by setting the block as free in the bitmap
 *          of blocks.
 * 
 * @param sb  Superblock in which the disk block should be freed.
 * @param num Number of the direct disk block that shall be freed.
 * 
 * @note The superblock must be locked.
 */
PRIVATE void block_free_direct(struct superblock *sb, block_t num)
{
	unsigned idx; /* Bitmap index.  */
	unsigned off; /* Bitmap offset. */
	
	/* Nothing to be done. */
	if (num == BLOCK_NULL)
		return;
		
	/* 
	 * Remember free disk block to
	 * speedup next block allocation.
	 */
	if (num < sb->zsearch)
		sb->zsearch = num;
	
	num -= sb->first_data_block;
	
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

/**
 * @brief Frees a single indirect disk block.
 * 
 * @details Frees a single indirect disk block by freeing all underlying direct
 *          disk blocks.
 * 
 * @param sb  Superblock in which the disk block should be freed.
 * @param num Number of the single indirect disk block that shall be freed.
 * 
 * @note The superblock must be locked.
 */
PRIVATE void block_free_indirect(struct superblock *sb, block_t num)
{
	unsigned i;         /* Loop index.   */
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

/**
 * @brief Frees a doubly indirect disk block.
 * 
 * @details Frees a doubly indirect disk block by freeing all underlying single 
 *          indirect disk blocks.
 * 
 * @param sb  Superblock in which the disk block should be freed.
 * @param num Number of the doubly indirect disk block that shall be freed.
 * 
 * @note The superblock must be locked.
 */
PRIVATE void block_free_dindirect(struct superblock *sb, block_t num)
{
	unsigned i;         /* Loop indexes. */
	struct buffer *buf; /* Block buffer. */
	
	/* Nothing to be done. */
	if (num == BLOCK_NULL)
		return;
	
	buf = bread(sb->dev, num);
		
	/* Free direct zone. */
	for (i = 0; i < NR_SINGLE; i++)
		block_free_indirect(sb, ((block_t *)buf->data)[i]);
	block_free_direct(sb, num);
	
	brelse(buf);
}

/**
 * @brief Frees a disk block.
 * 
 * @details Frees a disk block by freeing all underlying disk blocks.
 * 
 * @param sb  Superblock in which the disk block should be freed.
 * @param num Number of the disk block that shall be freed.
 * @param lvl Level of indirection to be parsed: zero for direct blocks, one for
 *        single indirect blocks, and two for doubly indirect blocks.
 * 
 * @note The superblock must be locked.
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
			kpanic("fs: bad indirection level");
	}
}

/**
 * @brief Maps a file byte offset in a disk block number.
 * 
 * @details Maps a file byte offset in a disk block number by traversing the
 *          disk block tree of the related inode. If the associated disk block
 *          does not exist, it may be created, if requested.
 * 
 * @param inode  Inode to use.
 * @param off    File offset.
 * @param create Zero if the block should be created if it does not exist, and
 *               one otherwise.
 * 
 * @returns Upon successful completion, the disk block number that is associated
 *          with the file byte offset is returned. Upon failure, #BLOCK_NULL is
 *          returned instead.
 * 
 * @note The inode must be locked.
 */
PUBLIC block_t block_map(struct inode *inode, off_t off, int create)
{
	block_t phys;       /* Physical block number. */
	block_t logic;      /* Logical block number.  */
	struct buffer *buf; /* Underlying buffer.     */
	
	logic = off/BLOCK_SIZE;
	
	/* File offset too big. */
	if (off/BLOCK_SIZE >= (ssize_t)(NR_DIRECT + NR_SINGLE + NR_DOUBLE))
	{
		curr_proc->errno = -EFBIG;
		return (BLOCK_NULL);
	}
	
	/* 
	 * Create blocks that are
	 * in a valid offset.
	 */
	if (off < inode->size)
		create = 1;
	
	/* Direct block. */
	if (logic < NR_ZONES_DIRECT)
	{
		/* Create direct block. */
		if (inode->blocks[logic] == BLOCK_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != BLOCK_NULL)
			{
				inode->blocks[logic] = phys;
				inode_touch(inode);
			}
		}
		
		return (inode->blocks[logic]);
	}
	
	logic -= NR_ZONES_DIRECT;
	
	/* Single indirect block. */
	if (logic < NR_SINGLE)
	{
		/* Create single indirect block. */
		if (inode->blocks[ZONE_SINGLE] == BLOCK_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != BLOCK_NULL)
			{
				inode->blocks[ZONE_SINGLE] = phys;
				inode_touch(inode);
			}
		}
		
		/* We cannot go any further. */
		if ((phys = inode->blocks[ZONE_SINGLE]) == BLOCK_NULL)
			return (BLOCK_NULL);
	
		buf = bread(inode->dev, phys);
		
		/* Create direct block. */
		if (((block_t *)buf->data)[logic] == BLOCK_NULL && create)
		{
			superblock_lock(inode->sb);
			phys = block_alloc(inode->sb);
			superblock_unlock(inode->sb);
			
			if (phys != BLOCK_NULL)
			{
				((block_t *)buf->data)[logic] = phys;
				buf->flags |= BUFFER_DIRTY;
				inode_touch(inode);
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
