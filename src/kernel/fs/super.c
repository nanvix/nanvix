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
#include <nanvix/klib.h>
#include <nanvix/fs.h>
#include <ustat.h>
#include "fs.h"

/**
 * @file
 *
 * @brief Superblock module implementation.
 */

/**
 * @addtogroup Superblock
 */
/**@{*/

/**
 * @brief Superblock table.
 *
 * @details The superblock table holds all information about mounted file
 *          systems.
 */
PRIVATE struct superblock superblocks[NR_SUPERBLOCKS];

/**
 * @brief Gets a non-valid superblock from the superblock table.
 *
 * @details Searches for a non-valid superblock entry in the superblock table.
 *
 * @returns If such superblock has been found, a pointer to it is returned. In
 *          this case, the superblock is ensured to be locked. However, if no
 *          such block has been found, a NULL pointer is returned instead.
 */
PRIVATE struct superblock *superblock_empty(void)
{
	struct superblock *sb;

again:

	/* Get empty superblock. */
	for (sb = &superblocks[0]; sb < &superblocks[NR_SUPERBLOCKS]; sb++)
	{
		/* Skip valid superblocks. */
		if (sb->flags & SUPERBLOCK_VALID)
			continue;

		/*
		 * Superblock is locked, so we
		 * wait for it to become free.
		 */
		if (sb->flags & SUPERBLOCK_LOCKED)
		{
			sleep(&sb->chain, PRIO_SUPERBLOCK);
			goto again;
		}

		superblock_lock(sb);
		return (sb);
	}

	/* Superblock table */
	kprintf("fs: superblock table overflow");

	return (NULL);
}

/**
 * @brief Writes superblock to underlying device.
 *
 * @details If the superblock is dirty, writes it to the underlying device.
 *          The inode and block maps are also written back.
 *
 * @param sb Superblock to be written back to disk.
 *
 * @note The superblock must be valid.
 * @note The superblock must be locked.
 */
PRIVATE void superblock_write(struct superblock *sb)
{
	/* Nothing to be done. */
	if (!(sb->flags & SUPERBLOCK_DIRTY))
		return;

	/* Write inode map buffers. */
	for (unsigned i = 0; i < sb->imap_blocks; i++)
	{
		sb->imap[i]->count++;
		bwrite(sb->imap[i]);
	}

	/* Write zone map buffers. */
	for (unsigned i = 0; i < sb->zmap_blocks; i++)
	{
		sb->zmap[i]->count++;
		bwrite(sb->zmap[i]);
	}

	/* Write superblock buffer. */
	sb->buf->count++;
	bwrite(sb->buf);

	sb->flags &= ~SUPERBLOCK_DIRTY;
}

/**
 * @brief Gets a superblock that matches a device number.
 *
 * @details Searches for a valid superblock in the superblock table which the
 *          device number equals the informed one.
 *
 * @param dev Device number.
 *
 * @returns If the requested superblock exists in the superblock table, a
 *          pointer to it is returned. In this case, the superblock is ensured
 *          to be locked. However, if no such superblock exists, a NULL pointer
 *          is returned instead.
 *
 * @note The device number should be valid.
 */
PUBLIC struct superblock *superblock_get(dev_t dev)
{
	struct superblock *sb;

again:

	/* Search for superblock. */
	for (sb = &superblocks[0]; sb < &superblocks[NR_SUPERBLOCKS]; sb++)
	{
		/* Skip invalid superblocks. */
		if (!(sb->flags & SUPERBLOCK_VALID))
			continue;

		/* Found. */
		if (sb->dev == dev)
		{
			/*
			 * Superblock is locked, so we
			 * wait for it to become free.
			 */
			if (sb->flags & SUPERBLOCK_LOCKED)
			{
				sleep(&sb->chain, PRIO_SUPERBLOCK);
				goto again;
			}

			superblock_lock(sb);
			sb->count++;
			return (sb);
		}
	}

	return (NULL);
}

/**
 * @brief Locks a superblock.
 *
 * @details Locks a superblock by marking it as locked. The calling process
 *          may block here some time, waiting its turn to acquire the lock.
 *
 * @param sb Superblock to be locked.
 *
 * @note The superblock must valid.
 */
PUBLIC void superblock_lock(struct superblock *sb)
{
	/* Waits for superblock to become unlocked. */
	while (sb->flags & SUPERBLOCK_LOCKED)
		sleep(&sb->chain, PRIO_SUPERBLOCK);

	sb->flags |= SUPERBLOCK_LOCKED;
}

/**
 * @brief Unlocks a superblock.
 *
 * @details Unlocks a superblock by marking it as not locked and waking up
 *          all processes that were waiting for it.
 *
 * @param sb Superblock to be unlocked.
 *
 * @note The superblock must be valid.
 * @note The superblock must be locked.
 */
PUBLIC void superblock_unlock(struct superblock *sb)
{
	wakeup(&sb->chain);
	sb->flags &= ~SUPERBLOCK_LOCKED;
}

/**
 * @brief Releases a superblock.
 *
 * @details Releases a superblock. If its reference count drops to zero, all
 *          underlying buffers are freed and then the superblock is marked as
 *          invalid.
 *
 * @param sb Superblock to be released.
 *
 * @note The superblock must be valid.
 * @note The superblock must be locked.
 */
PUBLIC void superblock_put(struct superblock *sb)
{
	/* Double free. */
	if (sb->count == 0)
		kpanic("freeing superblock twice");

	/* Release underlying resources. */
	if (--sb->count == 0)
	{
		superblock_write(sb);

		/* Release inode map buffers. */
		for (unsigned i = 0; i < sb->imap_blocks; i++)
			brelse(sb->imap[i]);

		/* Release zone map buffers. */
		for (unsigned i = 0; i < sb->zmap_blocks; i++)
			brelse(sb->zmap[i]);

		/* Release superblock buffer. */
		brelse(sb->buf);

		sb->flags &= ~SUPERBLOCK_VALID;
	}

	superblock_unlock(sb);
}

/**
 * @brief Reads a superblock from a device.
 *
 * @details Reads a superblock from the first block of a device. Once the read
 *          has completed, the magic number of the block is asserted and
 *          in-core fields are filled.
 *
 * @param dev Device number.
 *
 * @returns Upon successful completion, a pointer to the in-core superblock
 *          is returned. The superblock is ensured to be locked in this case.
 *          Upon failure, a NULL pointer is returned instead.
 *
 * @note The device number should be valid.
 *
 * @todo Check for read error on bread().
 */
PUBLIC struct superblock *superblock_read(dev_t dev)
{
	struct buffer *buf;        /* Buffer disk superblock. */
	struct superblock *sb;     /* In-core superblock.     */
	struct d_superblock *d_sb; /* Disk superblock.        */

	/* Get empty superblock. */
	sb = superblock_empty();
	if (sb == NULL)
		goto error0;

	/* Read superblock from device. */
	buf = bread(dev, 1);
	d_sb = (struct d_superblock *)buf->data;

	/* Bad magic number. */
	if (d_sb->s_magic != SUPER_MAGIC)
	{
		kprintf("fs: bad superblock magic number");
		goto error1;
	}

	/* Too many blocks in the inode/zone map. */
	if ((d_sb->s_imap_nblocks > IMAP_SIZE)||(d_sb->s_bmap_nblocks > ZMAP_SIZE))
	{
		kprintf("fs: too many blocks in the inode/zone map");
		goto error1;
	}

	/* Initialize superblock. */
	sb->buf = buf;
	sb->ninodes = d_sb->s_ninodes;
	sb->imap_blocks = d_sb->s_imap_nblocks;
	for (unsigned i = 0; i < sb->imap_blocks; i++)
		blkunlock(sb->imap[i] = bread(dev, 2 + i));
	sb->zmap_blocks = d_sb->s_bmap_nblocks;
	for (unsigned i = 0; i < sb->zmap_blocks; i++)
		blkunlock(sb->zmap[i] = bread(dev, 2 + sb->imap_blocks + i));
	sb->first_data_block = d_sb->s_first_data_block;
	sb->max_size = d_sb->s_max_size;
	sb->zones = d_sb->s_nblocks;
	sb->root = NULL;
	sb->mp = NULL;
	sb->dev = dev;
	sb->flags &= ~(SUPERBLOCK_DIRTY | SUPERBLOCK_RDONLY);
	sb->flags |= SUPERBLOCK_VALID;
	sb->isearch = 0;
	sb->zsearch = d_sb->s_first_data_block;
	sb->chain = NULL;
	sb->count++;

	blkunlock(buf);

	return (sb);

error1:
	brelse(buf);
	superblock_unlock(sb);
error0:
	return (NULL);
}

/**
 * @brief Synchronizes the superblock table.
 *
 * @details Synchronizes the superblock table by flushing all valid super
 *          blocks onto underlying devices.
 */
PUBLIC void superblock_sync(void)
{
	struct superblock *sb;

	/* Write superblocks to disk. */
	for (sb = &superblocks[0]; sb < &superblocks[NR_SUPERBLOCKS]; sb++)
	{
		superblock_lock(sb);

		/* Write only valid superblocks. */
		if (sb->flags & SUPERBLOCK_VALID)
			superblock_write(sb);

		superblock_unlock(sb);
	}
}

/**
 * @brief Gets file system statistics.
 *
 * @details Gets file system statics from the superblock of the file system
 *          pointed to by sb, and stores information in the buffer pointed to
 *          by ubuf.
 *
 * @param sb   Superblock of the file system to be inspected.
 * @param ubuf Place where statics should be stored.
 *
 * @note The superblock must be valid.
 * @note The superblock must be locked.
 * @note The buffer must be valid.
 */
PUBLIC void superblock_stat(struct superblock *sb, struct ustat *ubuf)
{
	int tfree;     /* Total free blocks. */
	int tinode;    /* Total free inodes. */
	int bmap_size; /* Size of block map. */
	int imap_size; /* Size of inode map. */

	/* Count number of free blocks. */
	tfree = 0;
	bmap_size = sb->zmap_blocks;
	for (int i = 0; i < bmap_size; i++)
	{
		for (int j = 0; j < (BLOCK_SIZE >> 2); j++)
			tfree += bitmap_nclear(sb->zmap[i]->data, BLOCK_SIZE);
	}

	/* Count number of free inodes. */
	tinode = 0;
	imap_size = sb->zmap_blocks;
	for (int i = 0; i < imap_size; i++)
	{
		for (int j = 0; j < (BLOCK_SIZE >> 2); j++)
			tinode += bitmap_nclear(sb->imap[i]->data, BLOCK_SIZE);
	}

	ubuf->f_tfree = tfree;
	ubuf->f_tinode = tinode;
	ubuf->f_fname[0] = '\0';
	ubuf->f_fpack[0] = '\0';
}

/**
 * @brief Initializes the superblock table.
 *
 * @details Initializes the superblock table by setting all superblocks in
 *          it the table to be invalid and unlocked.
 *
 * @note This function shall be called just once.
 */
PUBLIC void superblock_init(void)
{
	/* Initialize superblocks. */
	for (unsigned i = 0; i < NR_SUPERBLOCKS; i++)
	{
		superblocks[i].count = 0;
		superblocks[i].flags = ~(SUPERBLOCK_VALID | SUPERBLOCK_LOCKED |
			SUPERBLOCK_DIRTY | SUPERBLOCK_RDONLY);
	}

	kprintf("fs: superblock table has %d entries", NR_SUPERBLOCKS);
}

/**@}*/
