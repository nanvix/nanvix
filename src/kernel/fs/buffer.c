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
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include "fs.h"

/*
 * Too many buffers. The maximum value depends on
 * the amount of memory that is reserved to buffer
 * data. If you wanna change this, you shall take
 * a look on <nanvix/mm.h>
 */
#if (NR_BUFFERS > 512)
	#error "too many buffers"
#endif

/*
 * Number of buffers should be great enough so that
 * the superblock, the inode map and the free blocks
 * map do not waste more than 1/16 of buffers. If we
 * allowed that situation, we might observe a poor
 * performance.
 */
#if (IMAP_SIZE + ZMAP_SIZE > NR_BUFFERS/16)
	#error "hard disk too small"
#endif

/**
 * @brief Hash table size of the block buffer cache.
 */
#define BUFFERS_HASHTAB_SIZE 53

/**
 * @brief Block buffers.
 */
PRIVATE struct buffer buffers[NR_BUFFERS];

/**
 * @brief List of free block buffers.
 */
PRIVATE struct buffer free_buffers;

/**
 * @brief Processes waiting for any block.
 *
 * @details Chain of processes that are sleeping, waiting for any block to
 *          become free.
 */
PRIVATE struct process *chain = NULL;

/**
 * @brief block buffer hash table.
 */
PRIVATE struct buffer hashtab[BUFFERS_HASHTAB_SIZE];

/**
 * @brief Hash function for block buffer hash table.
 *
 * @details Hashes a device number and a block number to a block buffer hash
 *          table slot.
 */
#define HASH(dev, block) \
	(((dev)^(block))%BUFFERS_HASHTAB_SIZE)

/**
 * @brief Gets a block buffer from the block buffer cache.
 *
 * @details Searches the block buffer cache for a block buffer that matches
 *          a device number and block number.
 *
 * @param dev Device number.
 * @param num Block number.
 *
 * @returns Upon successful completion, a pointer to a buffer holding the
 *          requested block is returned. In this case, the block buffer is
 *          ensured to be locked, and may be, or may be not, valid.
 *          Upon failure, a null pointer NULL is returned instead.
 */
PRIVATE struct buffer *getblk(dev_t dev, block_t num)
{
	unsigned i;         /* Hash table index. */
	struct buffer *buf; /* Buffer.           */

	/* Should not happen. */
	if ((dev == 0) && (num == 0))
		kpanic("getblk(0, 0)");

repeat:

	i = HASH(dev, num);

	disable_interrupts();

	/* Search in hash table. */
	for (buf = hashtab[i].hash_next; buf != &hashtab[i]; buf = buf->hash_next)
	{
		/* Not found. */
		if ((buf->dev != dev) || (buf->num != num))
			continue;

		/*
		 * Buffer is locked so we wait for
		 * it to become free.
		 */
		if (buf->flags & BUFFER_LOCKED)
		{
			sleep(&buf->chain, PRIO_BUFFER);
			goto repeat;
		}

		/* Remove buffer from the free list. */
		if (buf->count++ == 0)
		{
			buf->free_prev->free_next = buf->free_next;
			buf->free_next->free_prev = buf->free_prev;
		}

		blklock(buf);
		enable_interrupts();

		return (buf);
	}

	/*
	 * There are no free buffers so we need to
	 * wait for one to become free.
	 */
	if (&free_buffers == free_buffers.free_next)
	{
		kprintf("fs: no free buffers");
		sleep(&chain, PRIO_BUFFER);
		goto repeat;
	}

	/* Remove buffer from the free list. */
	buf = free_buffers.free_next;
	buf->free_prev->free_next = buf->free_next;
	buf->free_next->free_prev = buf->free_prev;
	buf->count++;

	/*
	 * Buffer is dirty, so write it asynchronously
	 * to the disk and go find another buffer.
	 */
	if (buf->flags & BUFFER_DIRTY)
	{
		blklock(buf);
		enable_interrupts();
		bwrite(buf);
		goto repeat;
	}

	/* Remove buffer from hash queue. */
	buf->hash_prev->hash_next = buf->hash_next;
	buf->hash_next->hash_prev = buf->hash_prev;

	/* Reassign device and block number. */
	buf->dev = dev;
	buf->num = num;
	buf->flags &= ~BUFFER_VALID;

	/* Place buffer in a new hash queue. */
	hashtab[i].hash_next->hash_prev = buf;
	buf->hash_prev = &hashtab[i];
	buf->hash_next = hashtab[i].hash_next;
	hashtab[i].hash_next = buf;

	blklock(buf);
	enable_interrupts();

	return (buf);
}

/**
 * @brief Locks a block buffer.
 *
 * @details Locks the block buffer pointed to by buf by marking it as locked.
 *          The calling process may block here some time, waiting its turn to
 *          acquire the lock.
 *
 * @param buf Block buffer to be locked.
 */
PUBLIC void blklock(struct buffer *buf)
{
	disable_interrupts();

	/* Wait for block buffer to become unlocked. */
	while (buf->flags & BUFFER_LOCKED)
		sleep(&buf->chain, PRIO_BUFFER);

	buf->flags |= BUFFER_LOCKED;

	enable_interrupts();
}

/**
 * @brief Unlocks a block buffer.
 *
 * @details Unlocks the block buffer pointed to by buf by marking it as not
 *          locked and waking up all processes that were waiting for it.
 *
 * @param buf Block buffer to be unlocked.
 *
 * @note The block buffer must be locked.
 */
PUBLIC void blkunlock(struct buffer *buf)
{
	disable_interrupts();

	buf->flags &= ~BUFFER_LOCKED;
	wakeup(&buf->chain);

	enable_interrupts();
}

/**
 * @brief Puts back a block buffer in the block buffer cache.
 *
 * @details Releases the block buffer pointed to by buf. If its reference count
 *          drops to zero, the block buffer is put into the block buffer cache.
 *
 * @param buf Buffer to be released.
 *
 * @note The block buffer must be locked.
 */
PUBLIC void brelse(struct buffer *buf)
{
	disable_interrupts();

	/* Double free. */
	if (buf->count == 0)
		kpanic("freeing buffer twice");

	/* No more references. */
	if (--buf->count == 0)
	{
		/*
		 * Wakeup processes that were waiting
		 * for any block to become free.
		 */
		wakeup(&chain);

		/* Frequently used buffer (insert in the end). */
		if ((buf->flags & BUFFER_VALID) && (buf->flags & BUFFER_DIRTY))
		{
			free_buffers.free_prev->free_next = buf;
			buf->free_prev = free_buffers.free_prev;
			free_buffers.free_prev = buf;
			buf->free_next = &free_buffers;
		}

		/* Not frequently used buffer (insert in the begin). */
		else
		{
			free_buffers.free_next->free_prev = buf;
			buf->free_prev = &free_buffers;
			buf->free_next = free_buffers.free_next;
			free_buffers.free_next = buf;
		}
	}

	blkunlock(buf);
	enable_interrupts();
}

/**
 * @brief Reads a block from a device.
 *
 * @details Reads the block numbered num synchronously from the device numbered
 *          dev.
 *
 * @param dev Device number.
 * @param num Block number.
 *
 * @returns Upon successful completion, a pointer to a buffer holding the
 *          requested block is returned. In this case, the block buffer is
 *          ensured to be locked. Upon failure, a NULL pointer is returned
 *          instead.
 *
 * @note The device number should be valid.
 * @note The block number should be valid.
 */
PUBLIC struct buffer *bread(dev_t dev, block_t num)
{
	struct buffer *buf;

	buf = getblk(dev, num);

	/* Valid buffer? */
	if (buf->flags & BUFFER_VALID)
		return (buf);

	bdev_readblk(buf);

	/* Update buffer flags. */
	buf->flags |= BUFFER_VALID;
	buf->flags &= ~BUFFER_DIRTY;

	return (buf);
}

/**
 * @brief Writes a block buffer to the underlying device.
 *
 * @details Writes the block buffer pointed to by buf asynchronously to the
 *          underlying device. After that the operation has completed, the
 *          buffer is released.
 *
 * @param buf Block buffer to be written back.
 *
 * @note The block buffer must be locked.
 */
PUBLIC void bwrite(struct buffer *buf)
{
	/*
	 * We don't have to write back the buffer
	 * to disk, so we just release it and we are done.
	 */
	if (!(buf->flags & BUFFER_DIRTY))
	{
		brelse(buf);
		return;
	}

	/*
	 * The low-level I/O function shall clean
	 * the BUFFER_DIRTY flag and release the buffer.
	 */
	bdev_writeblk(buf);
}

/**
 * @brief Synchronizes the block buffer cache.
 *
 * @details Flushes all valid block buffers onto underlying devices.
 */
PUBLIC void bsync(void)
{
	/* Synchronize buffers. */
	for (struct buffer *buf = &buffers[0]; buf < &buffers[NR_BUFFERS]; buf++)
	{
		blklock(buf);

		/* Skip invalid buffers. */
		if (!(buf->flags & BUFFER_VALID))
		{
			blkunlock(buf);
			continue;
		}

		/*
		 * Prevent double free, since a call
		 * to brelse() will follow.
		 */
		disable_interrupts();
		if (buf->count++ == 0)
		{
			buf->free_prev->free_next = buf->free_next;
			buf->free_next->free_prev = buf->free_prev;
		}
		enable_interrupts();

		/*
		 * This will cause the buffer to be
		 * written back to disk and then released.
		 */
		bwrite(buf);
	}
}

/**
 * @brief Sets/clears buffer's dirty flag.
 *
 * @details If set equals to non-zero, then the dirty flag of the buffer pointed
 *          to by buf is set, otherwise the flag is cleared.
 *
 * @param buf Buffer in which the dirty flag shall be set/cleared.
 * @param set Set dirty flag?
 *
 * @note The buffer must be locked.
 */
PUBLIC inline void buffer_dirty(struct buffer *buf, int set)
{
	buf->flags = (set) ? buf->flags | BUFFER_DIRTY : buf->flags & ~BUFFER_DIRTY;
}

/**
 * @brief Returns a pointer to the data in a buffer.
 *
 * @details Returns a pointer to the data in the buffer pointed to by buf.
 *
 * @param buf Buffer to be considered.
 *
 * @returns A pointer to the data in the buffer.
 *
 * @note The buffer must be locked.
 */
PUBLIC inline void *buffer_data(const struct buffer *buf)
{
	return (buf->data);
}

/**
 * @brief Returns the device number of a buffer.
 *
 * @details Returns the device number of the buffer pointed to by buf.
 *
 * @param buf Buffer to be considered.
 *
 * @returns The device number of the buffer.
 *
 * @note The buffer must be locked.
 */
PUBLIC inline dev_t buffer_dev(const struct buffer *buf)
{
	return (buf->dev);
}

/**
 * @brief Returns the block number of a buffer.
 *
 * @details Returns the block number of the buffer pointed to by buf.
 *
 * @param buf Buffer to be considered.
 *
 * @returns The block number of the buffer.
 *
 * @note The buffer must be locked.
 */
PUBLIC inline block_t buffer_num(const struct buffer *buf)
{
	return (buf->num);
}

/**
 * @brief Asserts if a block buffer is marked as synchronous write.
 *
 * @details Asserts if the block buffer pointed to by buf is marked as
 *          synchronous write.
 *
 * @param buf Buffer to be asserted.
 *
 * @returns Non-zero if the buffer is marked as synchronous write, and zero
 *          otherwise.
 *
 * @note The buffer must be locked.
 */
PUBLIC inline int buffer_is_sync(const struct buffer *buf)
{
	return (buf->flags & BUFFER_SYNC);
}

/**
 * @brief Initializes the bock buffer cache.
 *
 * @details Initializes the block buffer cache by putting all buffers in the
 *          free list and cleaning the block buffer hash table.
 *
 * @note This function shall be called just once.
 */
PUBLIC void binit(void)
{
	char *ptr;

	kprintf("fs: initializing the block buffer cache");

	/* Initialize block buffers. */
	ptr = (char *)BUFFERS_VIRT;
	for (unsigned i = 0; i < NR_BUFFERS; i++)
	{
		buffers[i].dev = 0;
		buffers[i].num = 0;
		buffers[i].data = ptr;
		buffers[i].count = 0;
		buffers[i].flags =
			~(BUFFER_VALID | BUFFER_LOCKED | BUFFER_DIRTY | BUFFER_SYNC);
		buffers[i].chain = NULL;
		buffers[i].free_next =
			(i + 1 == NR_BUFFERS) ? &free_buffers : &buffers[i + 1];
		buffers[i].free_prev =
			(i == 0) ? &free_buffers : &buffers[i - 1];
		buffers[i].hash_next = &buffers[i];
		buffers[i].hash_prev = &buffers[i];

		ptr += BLOCK_SIZE;
	}

	/* Initialize the buffer cache. */
	free_buffers.free_next = &buffers[0];
	free_buffers.free_prev = &buffers[NR_BUFFERS - 1];
	for (unsigned i = 0; i < BUFFERS_HASHTAB_SIZE; i++)
	{
		hashtab[i].hash_prev = &hashtab[i];
		hashtab[i].hash_next = &hashtab[i];
	}

	kprintf("fs: %d slots in the block buffer cache", NR_BUFFERS);
}
