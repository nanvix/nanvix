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

#include <sys/types.h>
#include <fcntl.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "bitmap.h"
#include "minix.h"
#include "stat.h"
#include "util.h"

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
	size_t size;      /**< Size of bitmap (in byte). */
	uint32_t *bitmap; /**< Bitmap.                   */
} imap;

/**
 * @brief Zone map.
 */
static struct
{
	size_t size;      /**< Size of bitmap (in byte). */
	uint32_t *bitmap; /**< Bitmap.                   */
} zmap;

/**
 * @brief Reads the superblock of a Minix file system.
 */
static void minix_super_read(void)
{
	/* Read superblock. */
	slseek(fd, 1*BLOCK_SIZE, SEEK_SET);
	sread(fd, &super, sizeof(struct d_superblock));
	if (super.s_magic != SUPER_MAGIC)
		error("bad magic number");

	/* Read inode map. */
	slseek(fd, 2*BLOCK_SIZE, SEEK_SET);
	imap.size = super.s_imap_nblocks*BLOCK_SIZE;
	imap.bitmap = smalloc(imap.size);
	sread(fd, imap.bitmap, imap.size);

	/* Read block map. */
	slseek(fd, (2 + super.s_imap_nblocks)*BLOCK_SIZE, SEEK_SET);
	zmap.size = super.s_bmap_nblocks*BLOCK_SIZE;
	zmap.bitmap = smalloc(zmap.size);
	sread(fd, zmap.bitmap, zmap.size);
}

/**
 * @brief Writes the superblock of a Minix file system.
 */
static void minix_super_write(void)
{
	/* Write superblock. */
	slseek(fd, 1*BLOCK_SIZE, SEEK_SET);
	swrite(fd, &super, sizeof(struct d_superblock));

	/* Write inode map. */
	slseek(fd, 2*BLOCK_SIZE, SEEK_SET);
	swrite(fd, imap.bitmap, imap.size);

	/* Write zone map. */
	slseek(fd, (2 + super.s_imap_nblocks)*BLOCK_SIZE, SEEK_SET);
	swrite(fd, zmap.bitmap, zmap.size);

	/* House keeping. */
	free(imap.bitmap);
	free(zmap.bitmap);
}

/**
 * @brief Mounts a Minix file system.
 *
 * @param filename File where the Minix file system resides.
 */
void minix_mount(const char *filename)
{
	fd = sopen(filename, O_RDWR);
	minix_super_read();
}

/**
 * @brief Unmounts the currently mounted Minix file system.
 *
 * @note The Minix file system must be mounted.
 */
void minix_umount(void)
{
	minix_super_write();
	sclose(fd);
}

/**
 * @brief Reads an inode from the currently mounted Minix file system.
 *
 * @param num Number of the inode that shall be read.
 *
 * @returns A pointer to the requested inode.
 *
 * @note The Minix file system must be mounted.
 */
struct d_inode *minix_inode_read(uint16_t num)
{
	unsigned idx, off;         /* Inode number offset/index. */
	off_t offset;              /* Offset in the file system. */
	struct d_inode *ip;        /* Inode.                     */
	unsigned inodes_per_block; /* Inodes per block.          */

	num--;

	/* Bad inode number. */
	if (num >= super.s_imap_nblocks*BLOCK_SIZE*8)
		error("bad inode number");

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
 * @brief Writes an inode to the currently mounted Minix file system.
 *
 * @param num Number of the inode that shall be written.
 * @param ip  Inode that shall be written.
 *
 * @note The Minix file system must be mounted.
 */
void minix_inode_write(uint16_t num, struct d_inode *ip)
{
	unsigned idx, off;         /* Inode number offset/index. */
	off_t offset;              /* Offset in the file system. */
	unsigned inodes_per_block; /* Inodes per block.          */

	num--;

	/* Bad inode number. */
	if (num >= super.s_imap_nblocks*BLOCK_SIZE*8)
		error("bad inode number");

	/* Compute file offset. */
	inodes_per_block = BLOCK_SIZE/sizeof(struct d_inode);
	idx = num/inodes_per_block;
	off = num%inodes_per_block;
	offset = (2 + super.s_imap_nblocks + super.s_bmap_nblocks + idx)*BLOCK_SIZE;
	offset += off*sizeof(struct d_inode);

	/* Write inode. */
	slseek(fd, offset, SEEK_SET);
	swrite(fd, ip, sizeof(struct d_inode));

	free(ip);
}

/**
 * @brief Allocates a disk block.
 *
 * @returns The number of the allocated block.
 *
 * @note The Minix file system must be mounted.
 */
static block_t minix_block_alloc(void)
{
	uint32_t bit;

	/* Allocate block. */
	bit = bitmap_first_free(zmap.bitmap, zmap.size);
	if (bit == BITMAP_FULL)
		error("block map overflow");
	bitmap_set(zmap.bitmap, bit);

	return (super.s_first_data_block + bit);
}

/**
 * @brief Allocates an inode.
 *
 * @param mode Access mode.
 * @param uid  User ID.
 * @param gid  User group ID.
 *
 * @returns The number of the allocated inode.
 *
 * @note The Minix file system must be mounted.
 */
static uint16_t minix_inode_alloc(uint16_t mode, uint16_t uid, uint16_t gid)
{
	uint16_t num;       /* Inode number.                */
	uint32_t bit;       /* Bit number if the inode map. */
	struct d_inode *ip; /* New inode.                   */

	/* Allocate inode. */
	bit = bitmap_first_free(imap.bitmap, imap.size);
	if (bit == BITMAP_FULL)
		error("inode map overflow");
	bitmap_set(imap.bitmap, bit);
	num = bit + 1;

	/* Initialize inode. */
	ip = minix_inode_read(num);
	ip->i_mode = mode;
	ip->i_uid = uid;
	ip->i_size = 0;
	ip->i_time = 0;
	ip->i_gid = gid;
	ip->i_nlinks = 1;
	for (unsigned i = 0; i < NR_ZONES; i++)
		ip->i_zones[i] = BLOCK_NULL;
	minix_inode_write(num, ip);

	return (num);
}

/**
 * @brief Maps a file byte offset in a block number.
 *
 * @param ip     File to use
 * @param off    File byte offset.
 * @param create Create offset?
 *
 * @returns The block number that is associated with the file byte offset.
 *
 * @note @p ip must point to a valid inode.
 * @note The Minix file system must be mounted.
 */
static block_t minix_block_map(struct d_inode *ip, off_t off, bool create)
{
	block_t phys;                            /* Phys. blk. #.   */
	block_t logic;                           /* Logic. blk. #.  */
	block_t buf[BLOCK_SIZE/sizeof(block_t)]; /* Working buffer. */

	logic = off/BLOCK_SIZE;

	/* File offset too big. */
	if ((uint32_t)off >= super.s_max_size)
		error("file too big");

	/*
	 * Create blocks that are
	 * in a valid offset.
	 */
	if ((uint32_t)off < ip->i_size)
		create = true;

	/* Direct block. */
	if (logic < NR_ZONES_DIRECT)
	{
		/* Create direct block. */
		if (ip->i_zones[logic] == BLOCK_NULL && create)
		{
			phys = minix_block_alloc();
			ip->i_zones[logic] = phys;
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
			ip->i_zones[ZONE_SINGLE] = phys;
		}

		/* We cannot go any further. */
		if ((phys = ip->i_zones[ZONE_SINGLE]) == BLOCK_NULL)
			error("invalid offset");

		off = phys*BLOCK_SIZE;
		slseek(fd, off, SEEK_SET);
		sread(fd, buf, BLOCK_SIZE);

		/* Create direct block. */
		if (buf[logic] == BLOCK_NULL && create)
		{
			phys = minix_block_alloc();
			buf[logic] = phys;
			slseek(fd, off, SEEK_SET);
			swrite(fd, buf, BLOCK_SIZE);
		}

		return (buf[logic]);
	}

	logic -= NR_SINGLE;

	/* Double indirect zone. */
	error("double indect zone");

	return (BLOCK_NULL);
}

/**
 * @brief Searches for a directory entry.
 *
 * @param ip       Directory where the directory entry shall be searched.
 * @param filename Name of the directory entry that shall be searched.
 * @param create   Create directory entry?
 *
 * @returns The file offset where the directory entry is located, or -1 if the
 *          file does not exist.
 *
 * @note @p ip must point to a valid inode
 * @note @p filename must point to a valid file name.
 * @note The Minix file system must be mounted.
 */
static off_t dirent_search(struct d_inode *ip, const char *filename,bool create)
{
	int i;             /* Working entry.               */
	off_t base, off;   /* Working file offsets.        */
	int entry;         /* Free entry.                  */
	block_t blk;       /* Working block.               */
	int nentries;      /* Number of directory entries. */
	struct d_dirent d; /* Working directory entry.     */

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
			blk = minix_block_map(ip, i*sizeof(struct d_dirent), false);
			continue;
		}

		/* Compute file offset. */
		if (base < 0)
		{
			off = 0;
			base = blk*BLOCK_SIZE;
			slseek(fd, base, SEEK_SET);
		}

		/* Get next block. */
		else if (off >= BLOCK_SIZE)
		{
			base = -1;
			blk = minix_block_map(ip, i*sizeof(struct d_dirent), false);
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
					error("duplicate entry");

				return (base + off);
			}
		}

		/* Remember entry index. */
		else
			entry = i;

		i++;
		off += sizeof(struct d_dirent);
	}

	/* No entry found. */
	if (!create)
		return (-1);

	/* Expand directory. */
	if (entry < 0)
	{
		entry = nentries;
		blk = minix_block_map(ip, entry*sizeof(struct d_dirent), true);
		ip->i_size += sizeof(struct d_dirent);
	}

	else
		blk = minix_block_map(ip, entry*sizeof(struct d_dirent), false);

	/* Compute file offset. */
	off = (entry%(BLOCK_SIZE/sizeof(struct d_dirent)))*sizeof(struct d_dirent);
	base = blk*BLOCK_SIZE;

	return (base + off);
}

/**
 * @brief Searches for a file in a directory.
 *
 * @param dip       Directory where the file shall be searched.
 * @param filename File that shal be searched.
 *
 * @returns The inode number of the requested file, or #INODE_NULL, if such
 *          files does not exist.
 *
 * @note @p dip must point to a valid inode.
 * @note @p filename must point to a valid file name.
 * @note The Minix file system must be mounted.
 */
uint16_t dir_search(struct d_inode *dip, const char *filename)
{
	off_t off;         /* File offset where the entry is. */
	struct d_dirent d; /* Working directory entry.        */

	/* Not a directory. */
	if (!S_ISDIR(dip->i_mode))
		error("not a directory");

	/* Search directory entry. */
	off = dirent_search(dip, filename, false);
	if (off == -1)
		return (INODE_NULL);

	slseek(fd, off, SEEK_SET);
	sread(fd, &d, sizeof(struct d_dirent));

	return (d.d_ino);
}

/**
 * @brief Adds an entry in a directory.
 *
 * @param dip       Directory where the entry should be added.
 * @param filename Name of the entry.
 * @param num      Inode number of the entry.
 *
 * @note @p dip must point to a valid inode.
 * @note @p filename must point to a valid file name.
 * @note The Minix file system must be mounted.
 */
static void minix_dirent_add
(struct d_inode *dip, const char *filename, uint16_t num)
{
	off_t off;         /* File offset of the entry. */
	struct d_dirent d; /* Directory entry.          */

	/* Get free entry. */
	off = dirent_search(dip, filename, true);

	/* Read directory entry. */
	slseek(fd, off, SEEK_SET);
	sread(fd, &d, sizeof(struct d_dirent));

	/* Set attributes. */
	d.d_ino = num;
	strncpy(d.d_name, filename, MINIX_NAME_MAX);

	/* Write directory entry. */
	slseek(fd, off, SEEK_SET);
	swrite(fd, &d, sizeof(struct d_dirent));

	dip->i_nlinks++;
	dip->i_time = 0;
}

/**
 * @brief Gets inode number of the bottom-most directory of a path.
 *
 * @param pathname Path name that shall be converted.
 * @param filename Place to save last component of path.
 *
 * @returns The inode number of the bottom-most directory of the path.
 *
 * @note @p pathname must point to a valid path name.
 * @note The Minix file system must be mounted.
 */
uint16_t minix_inode_dname(const char *pathname, char *filename)
{
	struct d_inode *ip;   /* Working inode.         */
	uint16_t num1, num2;  /* Working inode numbers. */

	/* Traverse file system tree. */
	ip = minix_inode_read(num1 = INODE_ROOT);
	do
	{
		pathname = break_path(pathname, filename);
		num2 = dir_search(ip, filename);

		/* Bottom-most directory reached. */
		if (num2 == INODE_NULL)
		{
			/* Bad path. */
			if (*pathname != '\0')
				error("is a directory");

			goto out;
		}

		minix_inode_write(num1, ip);
		ip = minix_inode_read(num1 = num2);
	} while (*pathname != '\0');

out:
	minix_inode_write(num1, ip);

	return (num1);
}

/**
 * @brief Creates a directory.
 *
 * @param dip      Directory where the new directory shall be created.
 * @param dnum     Inode number of @p dip.
 * @param filename Name of the new directory.
 * @param uid  User ID.
 * @param gid  User group ID.
 *
 * @returns The inode number of the newly created directory.
 *
 * @note @p dip must point to a valid inode.
 * @note @p filename must point to a valid file name.
 * @note The Minix file system must be mounted.
 */
uint16_t minix_mkdir
(struct d_inode *dip, uint16_t dnum, const char *filename, uint16_t uid, uint16_t gid)
{
	uint16_t mode;
	uint16_t num;
	struct d_inode *ip;

	/* Not a directory. */
	if (!S_ISDIR(dip->i_mode))
		error("not a directory");

	mode  = S_IFDIR;
	mode |= S_IRWXU | S_IRGRP | S_IXGRP | S_IROTH | S_IXOTH;

	/* Allocate inode. */
	num = minix_inode_alloc(mode, uid, gid);
	minix_dirent_add(dip, filename, num);

	/* Create "." and ".." */
	ip = minix_inode_read(num);
	minix_dirent_add(ip, ".", num);
	minix_dirent_add(ip, "..", dnum);
	ip->i_nlinks--;
	minix_inode_write(num, ip);

	return (num);
}

/**
 * @brief Creates a special file.
 *
 * @param dip      Directory where the new special file shall be created.
 * @param filename Name of the new special file.
 * @param mode     Access mode to new special file.
 * @param dev      Device which the new special file refers to.
 * @param uid  User ID.
 * @param gid  User group ID.
 *
 * @note @p dip must point to a valid inode.
 * @note @p filename must point to a valid file name.
 * @note The Minix file system must be mounted.
 */
void minix_mknod
(struct d_inode *dip, const char *filename, uint16_t mode, uint16_t dev, uint16_t uid, uint16_t gid)
{
	uint16_t num;       /* Inode number of special file. */
	struct d_inode *ip; /* Special file.                 */

	/* Not a directory. */
	if (!S_ISDIR(dip->i_mode))
		error("not a directory");

	mode = (mode & ~S_IFMT) | ((dev & 1) ? S_IFBLK : S_IFCHR);

	/* Allocate inode. */
	num = minix_inode_alloc(mode, uid, gid);
	minix_dirent_add(dip, filename, num);

	/* Write device number. */
	ip = minix_inode_read(num);
	ip->i_zones[0] = dev;
	minix_inode_write(num, ip);
}

/**
 * @brief Creates a file.
 *
 * @param pathname Name of the file that shall be created.
 * @param mode     File access mode.
 * @param uid  User ID.
 * @param gid  User group ID.
 *
 * @returns The inode number of newly created file.
 *
 * @note @p pathname must refer to a valid path name.
 * @note @p num must refer to a valid inode.
 * @note The Minix file system must be mounted.
 */
uint16_t minix_create
(const char *pathname, uint16_t mode, uint16_t uid, uint16_t gid)
{
	uint16_t num;                      /* Inode number of file. */
	uint16_t dnum;                     /* Inode number of directory. */
	struct d_inode *dip;               /* Directory.            */
	char filename[MINIX_NAME_MAX + 1]; /* Name of the file.     */

	mode = (mode & ~S_IFMT) | S_IFREG;

	dnum = minix_inode_dname(pathname, filename);
	dip = minix_inode_read(dnum);
	num = minix_inode_alloc(mode, uid, gid);
	minix_dirent_add(dip, filename, num);
	minix_inode_write(dnum, dip);

	return (num);
}

/**
 * @brief Writes data to a file.
 *
 * @param num Inode number of the file.
 * @param buf Buffer to be written.
 * @param n   Number of bytes to be written.
 *
 * @note num must refer  to a valid inode.
 * @note buf must point to a valid buffer.
 * @note The Minix file system must be mounted.
 */
void minix_write(uint16_t num, const void *buf, size_t n)
{
	const char *p;      /* Writing pointer.     */
	off_t off;          /* Current file offset. */
	struct d_inode *ip; /* File.                */

	p = buf;

	ip = minix_inode_read(num);

	off = ip->i_size;

	/* Write data. */
	for (size_t i = 0; i < n; /* noop */)
	{
		size_t blkoff;
		size_t chunk;
		uint16_t blk;

		blk = minix_block_map(ip, off, true);
		blkoff = off % BLOCK_SIZE;
		chunk = ((n - i) < (BLOCK_SIZE - blkoff)) ? n - i : BLOCK_SIZE - blkoff;

		/* Write data to file. */
		slseek(fd, blk*BLOCK_SIZE + blkoff, SEEK_SET);
		swrite(fd, p, chunk);

		ip->i_size += chunk;
		off += chunk;
		p += chunk;
		i += chunk;
	}

	minix_inode_write(num, ip);
}

/**
 * @brief Creates a Minix file system.
 *
 * @param diskfile File where the minix file system shall be created.
 * @param ninodes  Number of inodes.
 * @param nblocks  Number of blocks.
 * @param uid  User ID.
 * @param gid  User group ID.
 *
 * @note @p diskfile must refer to a valid file.
 * @note @p ninodes must be valid.
 * @note @p nblocks must be valid.
 */
void minix_mkfs
(const char *diskfile, uint16_t ninodes, uint16_t nblocks, uint16_t uid, uint16_t gid)
{
	size_t size;            /* Size of file system.            */
	char buf[BLOCK_SIZE];   /* Writing buffer.                 */
	uint16_t imap_nblocks;  /* Number of inodes map blocks.    */
	uint16_t bmap_nblocks;  /* Number of block map blocks.     */
	uint16_t inode_nblocks; /* Number of inode blocks.         */
	struct d_inode *root;   /* Root directory.                 */
	mode_t mode;            /* Access permissions to root dir. */
	uint16_t num;           /* Inode number of root directory. */

	fd = sopen(diskfile, O_RDWR | O_CREAT);

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
	size += nblocks;       /* data blocks  */
	size <<= BLOCK_SIZE_LOG2;

	/* Fill file system with zeros. */
	memset(buf, 0, BLOCK_SIZE);
	for (size_t i = 0; i < size; i += BLOCK_SIZE)
		swrite(fd, buf, BLOCK_SIZE);

	/* Write superblock. */
	super.s_ninodes = ninodes;
	super.s_nblocks = nblocks;
	super.s_imap_nblocks = imap_nblocks;
	super.s_bmap_nblocks = bmap_nblocks;
	super.s_first_data_block = 2 + imap_nblocks + bmap_nblocks + inode_nblocks;
	super.s_max_size = 532480;
	super.s_magic = SUPER_MAGIC;

	/* Create inode map. */
	imap.size = imap_nblocks*BLOCK_SIZE;
	imap.bitmap = scalloc(imap_nblocks, BLOCK_SIZE);

	/* Create block map. */
	zmap.size = bmap_nblocks*BLOCK_SIZE;
	zmap.bitmap = scalloc(bmap_nblocks, BLOCK_SIZE);

	/* Access permission to root directory. */
	mode  = S_IFDIR;
	mode |= S_IRWXU | S_IRGRP | S_IXGRP | S_IROTH | S_IXOTH;

	/* Create root directory. */
	num = minix_inode_alloc(mode, uid, gid);
	root = minix_inode_read(num);
	minix_dirent_add(root, ".", num);
	minix_dirent_add(root, "..", num);
	root->i_nlinks--;
	minix_inode_write(num, root);

	minix_umount();
}
