/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
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

#include <nanvix/klib.h>
#include <fcntl.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
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

/**
 * @brief Hash table size.
 */
#define HASHTAB_SIZE 227

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
 * @returns Upon successful completion zero is returned. Upon failure, non-zero
 * is returned instead.
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
	if (sb == NULL){
		goto error0;
	}
	
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

	return (0);

error1:
	superblock_put(sb);
error0:
	return (1);
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
 * 
 * @returns Upon successful completion zero is returned. Upon failure, non-zero
 * is returned instead.
 * 
 * @note The superblock must not be locked.
 * 
 * @todo Use isearch.
 */
PUBLIC int inode_alloc_minix(struct superblock *sb,struct inode *ip)
{
	ino_t num;        /* Inode number.             */
	bit_t bit;        /* Bit number in the bitmap. */
	unsigned i;       /* Number of current block.  */
	
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
	
	superblock_unlock(sb);

	return (0);
	
error0:
	superblock_unlock(sb);
	return (1);
}


PUBLIC int minix_mkfs
(const char *diskfile, uint16_t ninodes, uint16_t nblocks, uint16_t uid, uint16_t gid)
{
	size_t size;            		 /* Size of file system.	   	        	*/
	char buf[BLOCK_SIZE];   		 /* Writing buffer.      	       	   		*/
	uint16_t imap_nblocks;  		 /* Number of inodes map blocks. 	   		*/
	uint16_t bmap_nblocks;  		 /* Number of block map blocks.    			*/
	uint16_t inode_nblocks; 		 /* Number of inode blocks.        		 	*/
	mode_t mode;            		 /* Access permissions to root dir.		 	*/
	struct d_superblock * d_super; 	 /* Disk superblock 						*/
	struct superblock * super; 		 /* In core superblock						*/
	struct inode * ip;				 /* Pointeur to in core inode				*/
	struct inode inode;				 /* Inode									*/
	dev_t dev;	 					 /* Device on witch file system in created	*/
	uint32_t * p ;
	struct buffer *buff;        	 /* Buffer disk superblock. 				*/
	off_t of;						 /* Offset									*/
	uint32_t imap_bitmap[IMAP_SIZE]; /*Inode map								*/
	uint32_t zmap_bitmap[ZMAP_SIZE]; /*Zone map									*/

	/*recuperation of the device */
	ip = inode_name(diskfile);
	
	/*Problem with device inode */
	if (ip == NULL)
	{	
		kprintf("Mkfs:device inode not found\n");
		return 0;
	}
	
	/*check if it's a device*/
	if (!S_ISCHR(ip->mode) && !S_ISBLK(ip->mode))
	{
		inode_put(ip);
		kprintf ("Mkfs:what you provide is not a device\n");
		return 0;
	}

	dev = ip->blocks[0];
	inode_put(ip);
	#define ROUND(x) (((x) == 0) ? 1 : (x))
	
	/* Compute dimensions of file sytem. */
	imap_nblocks = ROUND(ninodes/(8*BLOCK_SIZE));
	bmap_nblocks = ROUND(nblocks/(8*BLOCK_SIZE));
	inode_nblocks = ROUND( (ninodes*sizeof(struct d_inode))/BLOCK_SIZE);
	
	/* Compute size of file system. */
	size  = 1;             /* boot block   */
	size += 1;             /* superblock   */
	size += imap_nblocks;  /* inode map    */
	size += bmap_nblocks;  /* block map    */
	size += inode_nblocks; /* inode blocks */
	size = nblocks - size;       /* data blocks  */
	size <<= BLOCK_SIZE_LOG2;
	
	kprintf ("Mkfs: Initialisation of the file system");
	/* Fill file system with zeros. */
	of = 0;
	kmemset(buf, 0, BLOCK_SIZE);
	for (size_t i = 0; i < size; i += BLOCK_SIZE) //size
	{
		bdev_write(dev, buf, BLOCK_SIZE, of);
		of += BLOCK_SIZE;
	}
	
	kprintf ("Mkfs: Initialize superblock");
	/* Initialize superblock. */
	buff = bread(dev,1);
	if (buff == NULL)
	{
		kprintf("Mkfs: Error of lecture of the buffer");
		return 0;
	}

	d_super = (struct d_superblock *)buffer_data(buff);
	d_super->s_ninodes = ninodes;
	d_super->s_nblocks = nblocks;
	d_super->s_imap_nblocks = imap_nblocks;
	d_super->s_bmap_nblocks = bmap_nblocks;
	d_super->s_first_data_block = 2 + imap_nblocks + bmap_nblocks + inode_nblocks;
	d_super->s_max_size = 67641344; // a changer non 
	d_super->s_magic = SUPER_MAGIC;
	buffer_dirty(buff, 1);
	bwrite(buff);
	
	/* Create inode map. */
	kprintf("Mkfs: Create inode map");
	for (unsigned i = 0; i < imap_nblocks; i++)
	{
		buff = bread(dev,2+i);
		p = (uint32_t *)buffer_data(buff);
		*p = imap_bitmap[i];
		buffer_dirty(buff, 1);
		bwrite(buff);
	}
	
	/* Create block map. */
	kprintf("Mkfs: Create block map");
	for (unsigned i = 0; i < bmap_nblocks; i++)
	{
		buff= bread(dev,2+imap_nblocks+i); 
		p = buffer_data(buff);
		*p = zmap_bitmap[i];
		buffer_dirty(buff, 1);
		bwrite(buff);
	}
	
	/* Access permission to root directory. */
	mode  = S_IFDIR;
	mode |= S_IRWXU | S_IRGRP | S_IXGRP | S_IROTH | S_IXOTH;
	
	/*Recuperation of the superblock*/
	super = superblock_read(dev);
	
	if (super == NULL)
	{
		kprintf("Mkfs: Echec of the recuperation of the super block"); 
		return 0;
	}

	superblock_unlock(super);

	/* Create root directory. */
	ip = &inode;
	if (inode_alloc_minix(super,ip))
	{
		kprintf ("Mkfs : Allocation failed");
		return 0;
	}	
	
	if (ip == NULL)
	{
		kprintf ("Mkfs : Allocation failed");
		inode_put(ip);
		return 0;
	}
	
	if (dir_add(ip, ip, "."))
	{
		kprintf("Mkfs: Root directory can not be created");
		inode_put(ip);
		return 0;
	}

	ip->nlinks++;
		
	if (dir_add(ip, ip, ".."))
	{
		kprintf("Mkfs: Parent directory can not be created");
		inode_put(ip);
		return 0;
	}
	ip->nlinks++;
	superblock_lock(super);
	super->root = ip;
	superblock_put(super);

	ip->mode = mode;
	ip->uid = uid;
	ip->gid = gid;

	inode_write_minix(ip);
	inode_put(ip);

	bsync();
	return 1;
}

/**
 * @brief Minix file system operations.
 */
PRIVATE struct super_operations super_o_minix = {
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
	&super_o_minix,
	"minix"
};

PUBLIC struct super_operations * so_minix(void){
	return &super_o_minix;
}

/**
 * @brief initialise the file system in the virtual file system.
 */

PUBLIC void init_minix (){
	if (fs_register( MINIX , &fs_minix))
		kpanic ("failed du register file system minix");
}
