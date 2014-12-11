/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <sys/types.h>
#include <assert.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "bitmap.h"
#include "minix.h"
#include "safe.h"

/**
 * @brief Mounted superblock.
 */
static struct d_superblock super;

/**
 * @brief Identifier of the file where the mounted Minix file system resides.
 */
static int fd = -1;

/**
 * @brief Inode map.
 */
static struct
{
	size_t size;     /**< Size of bitmap (in byte). */
	uint32_t *bitmap; /**< Bitmap.                  */
} imap;

/**
 * @brief Zone map.
 */
static struct
{
	size_t size;     /**< Size of bitmap (in byte). */
	uint32_t *bitmap; /**< Bitmap.                  */
} zmap;

/**
 * @brief Reads Minix superblock.
 * 
 * @details Reads the superblock of the mounted Minix file system
 * 
 * @returns Upon successful completion, zero is returned. Upon failure, a
 *          negative error code is returned instead.
 */
static int minix_super_read(void)
{
	/* Sanity check. */
	assert(fd >= 0);
	
	/* Read superblock. */
	slseek(fd, 1*BLOCK_SIZE, SEEK_SET);
	sread(fd, &super, sizeof(struct d_superblock));
	if (super.s_magic != SUPER_MAGIC)
	{
		fprintf(stderr, "bad magic number %x\n", super.s_magic);
		return (-1);
	}
	
	/* Read inode map. */
	imap.bitmap = smalloc(super.s_imap_nblocks*BLOCK_SIZE);
	imap.size = super.s_imap_nblocks*BLOCK_SIZE;
	sread(fd, imap.bitmap, super.s_imap_nblocks*BLOCK_SIZE);
	
	/* Read zone map. */
	zmap.bitmap = smalloc(super.s_bmap_nblocks*BLOCK_SIZE);
	zmap.size = super.s_bmap_nblocks*BLOCK_SIZE;
	sread(fd, zmap.bitmap, super.s_bmap_nblocks*BLOCK_SIZE);
	
	return (0);
}

/**
 * @brief Writes Minix superblock.
 * 
 * @details Writes the superblock of the mounted Minix file system.
 * 
 * @returns Upon successful completion, zero is returned. Upon failure, a
 *          negative error code is returned instead.
 */
static int minix_super_write(void)
{
	/* Sanity check. */
	assert(fd >= 0);
	
	/* Write superblock. */
	slseek(fd, 1*BLOCK_SIZE, SEEK_SET);
	swrite(fd, &super, sizeof(struct d_superblock));
	
	/* Read inode map. */
	imap.bitmap = smalloc(super.s_imap_nblocks*BLOCK_SIZE);
	imap.size = super.s_imap_nblocks*BLOCK_SIZE;
	swrite(fd, imap.bitmap, super.s_imap_nblocks*BLOCK_SIZE);
	
	/* Read zone map. */
	zmap.bitmap = smalloc(super.s_bmap_nblocks*BLOCK_SIZE);
	zmap.size = super.s_bmap_nblocks*BLOCK_SIZE;
	swrite(fd, zmap.bitmap, super.s_bmap_nblocks*BLOCK_SIZE);
	
	/* House keeping. */
	free(imap.bitmap);
	free(zmap.bitmap);
	
	return (0);
}

/**
 * @brief Mounts a Minix file system.
 * 
 * @details Mounts the Minix file system that resides in the file @p filename.
 * 
 * @param filename File where the Minix file system resides.
 */
void minix_mount(const char *filename)
{
	int ret;
	
	fd = sopen(filename, O_RDWR);
	
	/* Read the super block. */
	ret = minix_super_read();
	if (ret < 0)
	{
		fprintf(stderr, "cannot minix_mount()\n");
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Unmounts a Minix file system.
 * 
 * @details Unmounts the currently mounted Minix file system.
 */
void minix_umount(void)
{
	int ret;
	
	/* Write the super block. */
	ret = minix_super_write();
	if (ret < 0)
	{
		fprintf(stderr, "cannot minix_unmount()\n");
		exit(EXIT_FAILURE);
	}
	
	sclose(fd);
}

/**
 * @brief Reads an inode from the mounted Minix file system.
 * 
 * @details Reads the inode numbered @p num from the mounted Minix file system.
 * 
 * @param num Number of the inode that shall be read.
 * 
 * @returns A pointer to the requested inode is returned.
 */
struct d_inode *minix_inode_read(uint16_t num)
{
	unsigned idx, off;         /* Inode number offset/index. */
	off_t offset;              /* Offset in the file system. */
	struct d_inode *ip;        /* Inode.                     */
	unsigned inodes_per_block; /* Inodes per block.          */
	
	/* Sanity check. */
	assert(num < super.s_ninodes);
	
	ip = smalloc(sizeof(struct d_inode));
	
	/* Compute file offset. */
	inodes_per_block = BLOCK_SIZE/sizeof(struct d_inode);
	idx = num/inodes_per_block; 
	off = num%inodes_per_block;
	offset = (2 + super.s_imap_nblocks + super.s_bmap_nblocks + idx)*BLOCK_SIZE;
	offset += off*sizeof(struct d_inode);
	
	/* Read inode. */
	slseek(fd, offset, SEEK_SET);
	sread(fd, ip, sizeof(struct d_inode));
	
	return (ip);
}

/**
 * @brief Writes an inode to the mounted Minix file system.
 * 
 * @details Writes the inode numbered @p num and pointed to by @p ip to the
 *          mounted Minix file system.
 * 
 * @param num Number of the inode that shall be written.
 * @param ip  Inode that shall be written.
 */
void minix_inode_write(uint16_t num, struct d_inode *ip)
{
	unsigned idx, off;         /* Inode number offset/index. */
	off_t offset;              /* Offset in the file system. */
	unsigned inodes_per_block; /* Inodes per block.          */
	
	/* Sanity check. */
	assert(num < super.s_ninodes);
	assert(ip != NULL);
	
	/* Compute file offset. */
	inodes_per_block = BLOCK_SIZE/sizeof(struct d_inode);
	idx = num/inodes_per_block; 
	off = num%inodes_per_block;
	offset = (2 + super.s_imap_nblocks + super.s_bmap_nblocks + idx)*BLOCK_SIZE;
	offset += off*sizeof(struct d_inode);
	
	/* Read inode. */
	slseek(fd, offset, SEEK_SET);
	swrite(fd, ip, sizeof(struct d_inode));
	
	free(ip);
}

/**
 * @brief Allocates a disk block.
 * 
 * @details Allocates a disk block by searching in the bitmap of zones.
 * 
 * @return Upon successful completion, the zone number of the allocated zone
 *         is returned. Upon failed, #BLOCK_NULL is returned instead.
 */
static block_t minix_block_alloc(void)
{
	uint32_t bit;

	bit = bitmap_first_free(zmap.bitmap, zmap.size);
	if (bit == BITMAP_FULL)
		return (BLOCK_NULL);

	bitmap_set(zmap.bitmap, bit);
	
	return (super.s_first_data_block + bit);
}

static block_t minix_inode_alloc(void)
{
	uint32_t bit;

	bit = bitmap_first_free(imap.bitmap, imap.size);
	if (bit == BITMAP_FULL)
		return (BLOCK_NULL);

	bitmap_set(imap.bitmap, bit);
	
	return (2 + super.s_imap_nblocks + super.s_bmap_nblocks + bit);
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
 */
static block_t minix_block_map(struct d_inode *ip, off_t off, int create)
{
	block_t phys;                            /* Phys. blk. #.   */
	block_t logic;                           /* Logic. blk. #.  */
	block_t buf[BLOCK_SIZE/sizeof(block_t)]; /* Working buffer. */
	
	logic = off/BLOCK_SIZE;
	
	/* File offset too big. */
	if (off >= super.s_max_size)
		return (BLOCK_NULL);
	
	/* 
	 * Create blocks that are
	 * in a valid offset.
	 */
	if (off < ip->i_size)
		create = 1;
	
	/* Direct block. */
	if (logic < NR_ZONES_DIRECT)
	{
		/* Create direct block. */
		if (ip->i_zones[logic] == BLOCK_NULL && create)
		{
			phys = minix_block_alloc();
			
			if (phys != BLOCK_NULL)
			{
				ip->i_zones[logic] = phys;
				ip->i_time = 0;
			}
		}
		
		return (ip->i_zones[logic]);
	}
	
	logic -= NR_ZONES_DIRECT;
	
	/* Single indirect block. */
	if (logic < NR_SINGLE)
	{
		/* Create single indirect block. */
		if (ip->i_zones[ZONE_SINGLE] == BLOCK_NULL && create)
		{
			phys = minix_block_alloc();
			
			if (phys != BLOCK_NULL)
			{
				ip->i_zones[ZONE_SINGLE] = phys;
				ip->i_time = 0;
			}
		}
		
		/* We cannot go any further. */
		if ((phys = ip->i_zones[ZONE_SINGLE]) == BLOCK_NULL)
			return (BLOCK_NULL);
	
		off = phys*BLOCK_SIZE;
		slseek(fd, off, SEEK_SET);
		sread(fd, buf, BLOCK_SIZE);
		
		/* Create direct block. */
		if (buf[logic] == BLOCK_NULL && create)
		{
			phys = minix_block_alloc();
			
			if (phys != BLOCK_NULL)
			{
				buf[logic] = phys;
				ip->i_time = 0;
				
				slseek(fd, off, SEEK_SET);
				swrite(fd, buf, BLOCK_SIZE);
			}
		}
		
		return (buf[logic]);
	}
	
	logic -= NR_SINGLE;
	
	/* Double indirect zone. */
	fprintf(stderr, "double indirect zones not supported yet");
	
	return (BLOCK_NULL);
}

/**
 * @brief Searches for a directory entry.
 * 
 * @details Searches for a directory entry named @p filename in the directory
 *          pointed to by @p ip. If @p create is not zero and such file entry
 *          does not exist, the entry is created.
 * 
 * @param ip       Directory where the directory entry shall be searched. 
 * @param filename Name of the directory entry that shall be searched.
 * @param create   Create directory entry?
 * 
 * @returns Upon successful completion, the file offset where the directory 
 *          entry is located is returned. Upon failure, (off_t)-1 is returned
 *          instead.
 */
static off_t dirent_search(struct d_inode *ip, const char *filename, int create)
{
	int i;         /* Working entry.              */
	off_t base, off;   /* Working file offsets.        */
	int entry;         /* Free entry.                  */
	block_t blk; /* Working block.               */
	int nentries; /* Number of directory entries. */
	struct d_dirent d;   /* Working directory entry.     */
	
	nentries = ip->i_size/sizeof(struct d_dirent);
	
	/* Search for directory entry. */
	i = 0;
	entry = -1;
	blk = ip->i_zones[0];
	base = -1;
	while (i < nentries)
	{
		/* Skip invalid block. */
		if (blk == BLOCK_NULL)
		{
			i += BLOCK_SIZE/sizeof(struct d_dirent);
			blk = minix_block_map(ip, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		/* Compute file offset. */
		if (base < 0)
		{
			off = 0;
			base = (super.s_first_data_block + blk)*BLOCK_SIZE;
			slseek(fd, base, SEEK_SET);
		}
		
		/* Get next block. */
		else if (off >= BLOCK_SIZE)
		{
			base = -1;
			blk = minix_block_map(ip, i*sizeof(struct d_dirent), 0);
			continue;
		}
		
		sread(fd, &d, sizeof(struct d_dirent));
		
		/* Valid entry. */
		if (d.d_ino != INODE_NULL)
		{
			/* Found. */
			if (!strncmp(d.d_name, filename, MINIX_NAME_MAX))
			{
				/* Duplicate entry. */
				if (create)
					return (-1);
				
				return (base + off);
			}
		}
		
		/* Remember entry index. */
		else
			entry = i;
		
		i++;
		off += sizeof(struct d_dirent);
	}
	
	fprintf(stderr, "minix_inode_read: %u %u\n", ip->i_size, ip->i_zones[0]);	
	
	/* Create entry. */
	if (create)
	{	
	
		/* Expand directory. */
		if (entry < 0)
		{
			entry = nentries;
			
			/* Allocate block. */
			blk = minix_block_map(ip, entry*sizeof(struct d_dirent), 1);
			if (blk == BLOCK_NULL)
				return (-1);
			
			fprintf(stderr, "blk: %u\n", blk);
			ip->i_size += sizeof(struct d_dirent);
			ip->i_time = 0;
		}
		
		else
			blk = minix_block_map(ip, entry*sizeof(struct d_dirent), 0);
		
		/* Compute file offset. */
		off = (entry%(BLOCK_SIZE/sizeof(struct d_dirent)))*sizeof(struct d_dirent);
		base = blk*BLOCK_SIZE;
		
			fprintf(stderr, "ret: %zd, %zd, %u\n", base + off, base, blk);
		return (base + off);
	}
	
	return (-1);
}

/**
 * @brief Searches for a file in a directory.
 * 
 * @details Searches for a file named @p filename in the directory pointed to by
 *          @p ip.
 * 
 * @param ip       Directory where the file shall be searched.
 * @param filename File that shal be searched.
 * 
 * @returns If the file exits, its inode number is returned. If the file does 
 *          not exist, #INODE_NULL is returned instead.
 */
uint16_t dir_search(struct d_inode *ip, const char *filename)
{
	off_t off;       /* File offset where the entry is. */
	struct d_dirent d; /* Working directory entry.        */
	
	/* Search directory entry. */
	off = dirent_search(ip, filename, 0);
	if (off < 0)
		return (INODE_NULL);
	
	slseek(fd, off, SEEK_SET);
	sread(fd, &d, sizeof(struct d_dirent));
	
	return (d.d_ino);
}

uint16_t minix_mkdir(struct d_inode *ip, const char *filename)
{
	off_t off;         /* File offset where the entry is. */
	struct d_dirent d; /* Working directory entry.        */
	
	/* Search directory entry. */
	off = dirent_search(ip, filename, 1);
	if (off < 0)
		return (INODE_NULL);
	
	slseek(fd, off, SEEK_SET);
	sread(fd, &d, sizeof(struct d_dirent));
	
	if (d.d_ino != INODE_NULL)
		return (INODE_NULL);
		
	d.d_ino = minix_inode_alloc();
	if (d.d_ino == INODE_NULL)
		return (INODE_NULL);
	
	strncpy(d.d_name, filename, MINIX_NAME_MAX);
	
	slseek(fd, off, SEEK_SET);
	swrite(fd, &d, sizeof(struct d_dirent));
	
	return (d.d_ino);
}
