/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
 * 
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
#include "minix/minix.h"

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
	if (!sb->s_op||!sb->s_op->superblock_write)
		kpanic ("WRITE: super_operation not initialized");
	sb->s_op->superblock_write(sb);
	
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
	//superblock_put_minix(sb);
	if (!sb->s_op||!sb->s_op->superblock_put)
		kpanic ("super_operation not initialized");
	sb->s_op->superblock_put(sb);
	
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

	struct superblock *sb;     /* In-core superblock.     */
	
		
	/* Get empty superblock. */	
	sb = superblock_empty();
	if (sb == NULL)
		return NULL;
	// if (!sb->s_op||!sb->s_op->superblock_read)
	// 	kpanic ("super_operation not initialized");
	// sb->s_op->superblock_read(dev,sb);
	superblock_read_minix(dev,sb);

	return (sb);
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
	if (!sb->s_op||!sb->s_op->superblock_stat)
		kpanic ("super_operation not initialized");
	sb->s_op->superblock_stat(sb,ubuf);
	//superblock_stat_minix(sb,ubuf);
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
