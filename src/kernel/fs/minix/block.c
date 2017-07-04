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

#include <nanvix/const.h>
#include <nanvix/clock.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <errno.h>
//#include "fs.h"

/**
 * @file
 * 
 * @brief Superblock module implementation.
 */

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
		bit = bitmap_first_free(buffer_data(sb->zmap[blk]), BLOCK_SIZE);
		
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
	bitmap_set(buffer_data(sb->zmap[blk]), bit);
	buffer_dirty(sb->zmap[blk], 1);
	sb->flags |= SUPERBLOCK_DIRTY;
	
	/* Clean block to avoid security issues. */
	buf = bread(sb->dev, blk);	
	kmemset(buffer_data(buf), 0, BLOCK_SIZE);
	buffer_dirty(buf, 1);
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
	bitmap_clear(buffer_data(sb->zmap[idx]), off);
	buffer_dirty(sb->zmap[idx], 1);
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
		block_free_direct(sb, ((block_t *)buffer_data(buf))[i]);
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
		block_free_indirect(sb, ((block_t *)buffer_data(buf))[i]);
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
 * @brief Create an indirect block.
 *
 * @details Creates a indirect block and saves in @param dest.
 *
 * @param dest Destination buffer, It is the corresponding disk block to be changed.
 * @param ip File to use.
 * @param offset Offset to be calculated the block.
 * @param creat If 1, creates an block, otherwise, just return this value.
 *
 * @returns If successful, the block number created / obtained, otherwise BLOCK_NULL is 
 *          returned.
 *
 * @note @p ip must be locked.
 */
PUBLIC block_t create_indirect_block
(struct buffer *dest, struct inode *ip, off_t offset, int create)
{
	block_t phys; /* Physical block number. */

	if (((block_t *)buffer_data(dest))[offset] == BLOCK_NULL && create)
	{
		/* Allocate an block. */
		superblock_lock(ip->sb);
		phys = block_alloc(ip->sb);
		superblock_unlock(ip->sb);

		if (phys != BLOCK_NULL)
		{
			((block_t *)buffer_data(dest))[offset] = phys;
			buffer_dirty(dest, 1);
			inode_touch(ip);
			brelse(dest);
			return (phys);
		}
		else
		{
			brelse(dest);
			return (phys);
		}
	}
	else
	{
		brelse(dest);
		return ((block_t *)buffer_data(dest))[offset];
	}
}

/**
 * @brief Create an direct block.
 *
 * @details Creates a direct block and saves in @param dest.
 *
 * @param ip File to use.
 * @param offset Offset to be calculated the block.
 * @param creat If 1, creates an block, otherwise, just return this value.
 *
 * @returns If successful, the block number created/obtained, otherwise BLOCK_NULL is 
 *          returned.
 *
 * @note @p ip must be locked.
 */
PUBLIC block_t create_direct_block(struct inode *ip, off_t offset, int create)
{
	block_t phys; /* Physical block number. */

	if (ip->blocks[offset] == BLOCK_NULL && create)
	{
		/* Allocate an block. */
		superblock_lock(ip->sb);
		phys = block_alloc(ip->sb);
		superblock_unlock(ip->sb);

		if (phys != BLOCK_NULL)
		{
			ip->blocks[offset] = phys;
			inode_touch(ip);
			return (phys);
		}
		else
			return (phys);
	}
	else
		return (ip->blocks[offset]);
}

/**
 * @brief Maps a file byte offset in a disk block number.
 * 
 * @details Maps the offset @p off in the file pointed to by @p ip in a disk
 *          block number. If @p create is not zero and such file by offset is
 *          invalid, the file is expanded accordingly to make it valid.
 * 
 * @param ip     File to use
 * @param off    File byte offset.
 * @param create Create offset?
 * 
 * @returns Upon successful completion, the disk block number that is associated
 *          with the file byte offset is returned. Upon failure, #BLOCK_NULL is
 *          returned instead.
 * 
 * @note @p ip must be locked.
 */
PUBLIC block_t block_map(struct inode *ip, off_t off, int create)
{
	block_t phys;       /* Physical block number.    */
	block_t logic;      /* Logical block number.     */
	struct buffer *buf; /* Underlying buffer.        */
	unsigned tmp;       /* Logical block number tmp. */
	
	logic = off/BLOCK_SIZE;
	
	/* File offset too big. */
	if (off >= ip->sb->max_size)
	{
		curr_proc->errno = -EFBIG;
		return (BLOCK_NULL);
	}
	
	/* 
	 * Create blocks that are
	 * in a valid offset.
	 */
	if (off < ip->size)
		create = 1;
	
	/* Direct block. */
	if (logic < NR_ZONES_DIRECT)
	{
		/* Create direct block. */
		if (ip->blocks[logic] == BLOCK_NULL && create)
		{
			superblock_lock(ip->sb);
			phys = block_alloc(ip->sb);
			superblock_unlock(ip->sb);
			
			if (phys != BLOCK_NULL)
			{
				ip->blocks[logic] = phys;
				inode_touch(ip);
			}
		}
		
		return (ip->blocks[logic]);
	}
	
	logic -= NR_ZONES_DIRECT;
	
	/* Single indirect block. */
	if (logic < NR_SINGLE)
	{
		/* Create single indirect block. */
		if (ip->blocks[ZONE_SINGLE] == BLOCK_NULL && create)
		{
			superblock_lock(ip->sb);
			phys = block_alloc(ip->sb);
			superblock_unlock(ip->sb);
			
			if (phys != BLOCK_NULL)
			{
				ip->blocks[ZONE_SINGLE] = phys;
				inode_touch(ip);
			}
		}
		
		/* We cannot go any further. */
		if ((phys = ip->blocks[ZONE_SINGLE]) == BLOCK_NULL)
			return (BLOCK_NULL);
	
		buf = bread(ip->dev, phys);
		
		/* Create direct block. */
		if (((block_t *)buffer_data(buf))[logic] == BLOCK_NULL && create)
		{
			superblock_lock(ip->sb);
			phys = block_alloc(ip->sb);
			superblock_unlock(ip->sb);
			
			if (phys != BLOCK_NULL)
			{
				((block_t *)buffer_data(buf))[logic] = phys;
				buffer_dirty(buf, 1);
				inode_touch(ip);
			}
		}
		
		brelse(buf);
		
		return (((block_t *)buffer_data(buf))[logic]);
	}
	
	logic = off - REMAINING_OFFSET;
	
	/* Double indirect block. */
	tmp = logic / BLOCK_SIZE;
	if (tmp < NR_DOUBLE)
	{
		size_t logicSingle = logic / (NR_SINGLE * BLOCK_SIZE);
		size_t logicDouble = logic / BLOCK_SIZE;
		
		/* Create single, double and/or direct block. */
		if ( (phys = create_direct_block(ip,ZONE_DOUBLE,create)) != BLOCK_NULL)
		{
			buf = bread(ip->dev, phys);
			if ( (phys = create_indirect_block(buf,ip,logicSingle,create)) 
				!= BLOCK_NULL)
			{
				buf = bread(ip->dev, phys);
				if ( (phys = create_indirect_block(buf,ip,logicDouble,create)) 
					!= BLOCK_NULL)
					return (phys);
				else
					return (BLOCK_NULL);
			}
			else
				return (BLOCK_NULL);
		}
		else
			return (BLOCK_NULL);
	}

	/* Triple indirect zone. */
	kpanic("triple indirect zones not supported");
	
	return (BLOCK_NULL);
}

/**@}*/
