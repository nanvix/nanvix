/*
 * Copyright(C) 2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/cache.c - Block buffer cache library implementation.
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include "fs.h"

/* Number of block buffers (don't touch). */
#define NR_BUFFERS 256

#if (IMAP_SIZE + ZMAP_SIZE > NR_BUFFERS/16)
	#error "hard disk too big"
#endif

/* Block buffers hash table size. */
#define HASHTAB_SIZE 53

/* Block buffers. */
PRIVATE struct buffer buffers[NR_BUFFERS];

/* Block buffers hash table. */
PRIVATE struct buffer *hashtab[HASHTAB_SIZE];

/* Free block buffers. */
PRIVATE struct buffer *free_buffers;

/* Processes waiting for any block. */
PRIVATE struct process *chain = NULL;

/*
 * Hash function.
 */
#define HASH(dev, block) \
	(((dev)^(block))%HASHTAB_SIZE)

/*
 * Removes a block from the cache.
 */
PRIVATE struct buffer *cache_remove(struct buffer *buf)
{	
	if (buf == NULL)
		buf = free_buffers;
		
	/* No free buffers. */
	if (buf == NULL)
		return (NULL);
	
	/* Remove buffer from the free list. */
	if (buf->count == 0)
	{
		buf->free_prev->free_next = buf->free_next;
		buf->free_next->free_prev = buf->free_prev;
		if (buf->free_next == buf->free_prev)
			free_buffers = NULL;
		else if (free_buffers == buf)
			free_buffers = buf->free_next;
	}
	
	buf->count++;

	return (buf);
}

/*
 * Updates a block in the cache.
 */
PRIVATE void cache_update(struct buffer *buf, dev_t dev, block_t num)
{
	/* Remmove buffer from hash queue. */
	if (buf->hash_prev != NULL)
		buf->hash_prev->hash_next = buf->hash_next;
	if (buf->hash_next != NULL)
		buf->hash_next->hash_prev = buf->hash_prev;
	if (hashtab[HASH(buf->dev, buf->num)] == buf)
		hashtab[HASH(buf->dev, buf->num)] = buf->hash_next;
	
	buf->dev = dev;
	buf->num = num;
	buf->flags &= ~BUFFER_VALID;
	
	/* Insert buffer in a new hash queue. */
	buf->hash_next = hashtab[HASH(buf->dev, buf->num)];
	buf->hash_prev = NULL;
	hashtab[HASH(buf->dev, buf->num)] = buf;
	if (buf->hash_next != NULL)
		buf->hash_next->hash_prev = buf;
}

/*
 * Puts back a block buffer in the cache.
 */
PRIVATE void cache_put(struct buffer *buf)
{
	disable_interrupts();
	
	/* No more references. */
	if (--buf->count == 0)
	{	
		wakeup(&chain);
		
		/* Insert buffer in the free list. */
		if (free_buffers == NULL)
		{
			buf->free_next = buf;
			buf->free_prev = buf;
			free_buffers = buf;
		}
		
		else
		{	
			buf->free_next = free_buffers;
			buf->free_prev = free_buffers->free_prev;
					
			/* Frenquently used buffer (insert in the end). */
			if ((buf->flags & BUFFER_VALID) && !(buf->flags & BUFFER_DIRTY))
			{	
				free_buffers->free_prev->free_next = buf;
				free_buffers->free_prev = buf;
			}
			
			/* Not frequently used buffer (insert in the begin). */
			else
			{	
				free_buffers->free_next->free_prev = buf;
				free_buffers->free_prev = buf;	
				free_buffers = buf;
			}
		}
	}
	
	enable_interrupts();

	block_unlock(buf);
}

/*
 * Gets a block buffer from the cache.
 */
PRIVATE struct buffer *cache_get(dev_t dev, block_t num)
{
	struct buffer *buf;

repeat:

	disable_interrupts();

	/* Search in hash table. */
	for (buf = hashtab[HASH(dev, num)]; buf != NULL; buf = buf->hash_next)
	{
		/* Not found. */
		if ((buf->dev != dev) || (buf->num != num))
			continue;
		
		/* Buffer is locked. */
		if (buf->flags & BUFFER_LOCKED)
		{
			sleep(&buf->chain, PRIO_BUFFER);
			goto repeat;
		}
		
		cache_remove(buf);

		enable_interrupts();

		block_lock(buf);

		return (buf);
	}

	buf = cache_remove(NULL);

	/* There are no free buffers. */
	if (buf == NULL)
	{
		kprintf("fs: block buffer cache full");
		sleep(&chain, PRIO_BUFFER);
		goto repeat;
	}
	
	/* Synchronize buffer. */
	if (buf->flags & BUFFER_DIRTY)
	{
		enable_interrupts();
		block_lock(buf);
		bdev_writeblk(buf);
		goto repeat;
	}
	
	cache_update(buf, dev, num);
	
	enable_interrupts();
	
	block_lock(buf);
	
	return (buf);
}

/*
 * Synchronizes the cache.
 */
PUBLIC void cache_sync(void)
{
	struct buffer *buf;
	
	/* Synchronize buffers. */
	for (buf = &buffers[0]; buf < &buffers[NR_BUFFERS]; buf++)
	{
		block_lock(buf);
		
		/* Write only valid buffers. */
		if (buf->flags & BUFFER_VALID)
		{
			/* Write buffer to disk. */
			if (buf->flags & BUFFER_DIRTY)
				bdev_writeblk(buf);
		}
		
		block_unlock(buf);
	}
}

/*
 * Initializes the block buffer cache.
 */
PUBLIC void cache_init(void)
{
	int i;     /* Loop index.          */
	char *ptr; /* Buffer data pointer. */
	
	kprintf("fs: initializing the block buffer cache");
	
	ptr = (char *)BUFFERS_VIRT;
	
	/* Initialize buffers. */
	for (i = 0; i < NR_BUFFERS; i++)
	{
		buffers[i].data = ptr;
		buffers[i].count = 0;
		buffers[i].flags = ~(BUFFER_DIRTY | BUFFER_VALID | BUFFER_LOCKED);
		buffers[i].chain = NULL;
		buffers[i].free_next = &buffers[(i + 1)%NR_BUFFERS];
		buffers[i].free_prev = &buffers[(i - 1)%NR_BUFFERS];
		buffers[i].hash_next = NULL;
		buffers[i].hash_prev = NULL;
		
		ptr += BLOCK_SIZE;
	}
	
	/* Initialize buffer cache. */
	free_buffers = &buffers[0];
	free_buffers->free_prev = &buffers[NR_BUFFERS - 1];
	for (i = 0; i < HASHTAB_SIZE; i++)
		hashtab[i] = NULL;
}

/*
 * Locks a block buffer.
 */
PUBLIC void block_lock(struct buffer *buf)
{
	disable_interrupts();

	/* Wait for block buffer to become unlocked. */
	while (buf->flags & BUFFER_LOCKED)
		sleep(&buf->chain, PRIO_BUFFER);
	
	buf->flags |= BUFFER_LOCKED;

	enable_interrupts();
}

/*
 * Unlocks a block buffer.
 */
PUBLIC void block_unlock(struct buffer *buf)
{
	disable_interrupts();

	wakeup(&buf->chain);
	buf->flags &= ~BUFFER_LOCKED;

	enable_interrupts();
}

/*
 * Releases access to a block buffer.
 */
PUBLIC void block_put(struct buffer *buf)
{
	cache_put(buf);
}

/*
 * Reads a block buffer.
 */
PUBLIC struct buffer *block_read(dev_t dev, block_t num)
{
	struct buffer *buf;
	
	buf = cache_get(dev, num);
	
	/* Valid buffer? */
	if (buf->flags & BUFFER_VALID)
		return (buf);
	
	bdev_readblk(buf);
	
	return (buf);
}

/*
 * Writes a block buffer.
 */
PUBLIC void block_write(struct buffer *buf)
{
	/* Write block. */
	if (buf->flags & BUFFER_DIRTY)
		bdev_writeblk(buf);
	
	cache_put(buf);
}
