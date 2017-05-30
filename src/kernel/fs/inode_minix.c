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
#include "inode_minix.h"

/* Number of inodes per block. */
#define INODES_PER_BLOCK (BLOCK_SIZE/sizeof(struct d_inode))

/**
 * @brief In-core inodes table.
 */
//PRIVATE struct inode inodes[NR_INODES];

/**
 * @brief Hash table size.
 */
#define HASHTAB_SIZE 227

/* Free inodes. */




/**
 * @brief Hash function for the inode cache.
 */
#define HASH(dev, num) \
	(((dev)^(num))%HASHTAB_SIZE)







/**
 * @brief Writes an inode to disk.
 * 
 * @details Writes the inode pointed to by @p ip to disk.
 * 
 * @param ip Inode to be written to disk.
 * 
 * @note The inode must be locked.
 */
PUBLIC void inode_write_minix(struct inode *ip)
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
	
	d_i = &(((struct d_inode *)buffer_data(buf))[(ip->num - 1)%INODES_PER_BLOCK]);
	
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
PUBLIC int inode_read_minix(dev_t dev, ino_t num, struct inode *ip)
{
	block_t blk;           /* Block.         */
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
	
	d_i = &(((struct d_inode *)buffer_data(buf))[(num - 1)%INODES_PER_BLOCK]);
	
	/* Invalid disk inode. */ 
	if (d_i->i_nlinks == 0)
		goto error1;
	


	// TODO a enlever
	/* Get a free in-core inode. */
	//ip = inode_cache_evict();
	

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
	return 1;
	

error1:
	superblock_put(sb);
	return 1;
error0:
	//TODO traiter cette erreur autrement
	//return (NULL);
	return 0;
	kprintf("Problemme dans inode_minix.c ligne 195\n");
	
}

/**
 * @brief Frees an inode.
 * 
 * @details Frees the inode pointed to by @p ip.
 * 
 * @details The inode must be locked.
 */
PUBLIC void inode_free_minix(struct inode *ip)
{
	block_t blk;           /* Block number.           */
	struct superblock *sb; /* Underlying super block. */
	
	blk = (ip->num - 1)/(BLOCK_SIZE << 3);
	
	superblock_lock(sb = ip->sb);
	
	bitmap_clear(buffer_data(sb->imap[blk]), (ip->num - 1)%(BLOCK_SIZE << 3));
	
	buffer_dirty(sb->imap[blk], 1);
	if (ip->num < sb->isearch)
		sb->isearch = ip->num;
	sb->flags |= SUPERBLOCK_DIRTY;
	
	superblock_unlock(sb);
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
PUBLIC void inode_truncate_minix(struct inode *ip)
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
PUBLIC int inode_alloc_minix(struct superblock *sb,struct inode *ip )
{
	ino_t num;        /* Inode number.             */
	bit_t bit;        /* Bit number in the bitmap. */
	unsigned i;       /* Number of current block.  */

	ip=NULL;
	
	superblock_lock(sb);
	
	/* Search for free inode. */
	for (i = 0; i < sb->imap_blocks; i++)
	{
		bit = bitmap_first_free(buffer_data(sb->imap[i]), BLOCK_SIZE);
		
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

	//TODO a enlever
	/* Get a free in-core inode. */
	//ip = inode_cache_evict();
	
	if (ip == NULL)
		goto error0;
	
	/* Allocate inode. */
	bitmap_set(buffer_data(sb->imap[i]), bit);
	buffer_dirty(sb->imap[i], 1);
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
	
	//inode_touch(ip);
	//inode_cache_insert(ip);
	ip=NULL;
	
	superblock_unlock(sb);
	return 1;
	
	
error0:
	superblock_unlock(sb);
	return 0;
	kprintf ("problemme a traiter dans inode_minix ligne 361\n");
}

PRIVATE struct super_operations so_minix = {
		&inode_read_minix, 
		&inode_write_minix,
		&inode_free_minix,
		&inode_truncate_minix,
		&inode_alloc_minix,
		NULL,
		NULL,
		&superblock_put,
		&superblock_put,
		&init_minix
};

PRIVATE struct file_system_type fs_minix = {
	NULL,
	&so_minix,
	"minix"
};

/**
 * @brief initialise the file system in the virtual file system.
 */

PUBLIC void init_minix (){
	fs_register( MINIX , &fs_minix);
}