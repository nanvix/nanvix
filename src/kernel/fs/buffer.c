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

/* Too many buffers. */
#if (NR_BUFFERS > 512)
	#error "too many buffers"
#endif

/* Hard disk too big. */
#if (IMAP_SIZE + ZMAP_SIZE > NR_BUFFERS/16)
	#error "hard disk too big"
#endif

/* Block buffers. */
PRIVATE struct buffer buffers[NR_BUFFERS];

/* Block buffers hash table. */
PRIVATE struct buffer hashtab[BUFFERS_HASHTAB_SIZE];

/* Free block buffers. */
PRIVATE struct buffer *free_buffers = NULL;

/* Processes waiting for any block. */
PRIVATE struct process *chain = NULL;

/*
 * Hash function.
 */
#define HASH(dev, block) \
	(((dev)^(block))%BUFFERS_HASHTAB_SIZE)

/*
 * Gets a block buffer from the cache.
 */
PRIVATE struct buffer *getblk(dev_t dev, block_t num)
{
	int i;              /* Hash table index. */
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
		
		enable_interrupts();
		blklock(buf);

		return (buf);
	}

	buf = free_buffers->free_next;

	/*
	 * There are no free buffers so we need to
	 * wait for one to become free.
	 */
	if (buf == buf->free_next)
	{
		kprintf("fs: block buffer cache full");
		sleep(&chain, PRIO_BUFFER);
		goto repeat;
	}
	
	/* Remove buffer from the free list. */
	buf->free_prev->free_next = buf->free_next;
	buf->free_next->free_prev = buf->free_prev;
	buf->count++;
	
	/* 
	 * Buffer is dirty, so write it asynchronously 
	 * to the disk and go get another one.
	 */
	if (buf->flags & BUFFER_DIRTY)
	{
		enable_interrupts();
		blklock(buf);
		bdev_writeblk(buf);
		goto repeat;
	}
	
	/* Remove buffer from hash queue. */
	buf->hash_prev->hash_next = buf->hash_next;
	buf->hash_next->hash_prev = buf->hash_prev;
	
	/* Reassigns device and block number. */
	buf->dev = dev;
	buf->num = num;
	buf->flags &= ~BUFFER_VALID;
	
	/* Place buffer in a new hash queue. */
	hashtab[i].hash_next->hash_prev = buf;
	buf->hash_prev = &hashtab[i];
	buf->hash_next = hashtab[i].hash_next;
	hashtab[i].hash_next = buf;
	
	enable_interrupts();
	blklock(buf);
	
	return (buf);
}

/*
 * Locks a block buffer.
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

/*
 * Unlocks a block buffer.
 */
PUBLIC void blkunlock(struct buffer *buf)
{
	disable_interrupts();

	buf->flags &= ~BUFFER_LOCKED;
	wakeup(&buf->chain);

	enable_interrupts();
}

/*
 * Puts back a block buffer in the cache.
 */
PUBLIC void brelse(struct buffer *buf)
{
	disable_interrupts();
	
	/* No more references. */
	if (--buf->count == 0)
	{	
		wakeup(&chain);
		
		buf->free_next = free_buffers;
		buf->free_prev = free_buffers->free_prev;
					
		/* Frequently used buffer (insert in the end). */
		if ((buf->flags & BUFFER_VALID) && (buf->flags & BUFFER_DIRTY))
		{	
			free_buffers->free_prev->free_next = buf;
			buf->free_prev = free_buffers->free_prev;
			free_buffers->free_prev = buf;
			buf->free_next = free_buffers;
		}
			
		/* Not frequently used buffer (insert in the begin). */
		else
		{	
			free_buffers->free_next->free_prev = buf;
			buf->free_prev = free_buffers;
			buf->free_next = free_buffers->free_next;
			free_buffers->free_next = buf;
		}
	}
	
	enable_interrupts();

	blkunlock(buf);
}

/*
 * Reads a block buffer.
 */
PUBLIC struct buffer *bread(dev_t dev, block_t num)
{
	struct buffer *buf;
	
	buf = getblk(dev, num);
	
	/* Valid buffer? */
	if (buf->flags & BUFFER_VALID)
		return (buf);

	bdev_readblk(buf);
	
	return (buf);
}

/*
 * Writes a block buffer.
 */
PUBLIC void bwrite(struct buffer *buf)
{
	bdev_writeblk(buf);
}

/*
 * Synchronizes the cache.
 */
PUBLIC void bsync(void)
{
	struct buffer *buf;
	
	/* Synchronize buffers. */
	for (buf = &buffers[0]; buf < &buffers[NR_BUFFERS]; buf++)
	{
		blklock(buf);
			
		/* Skip invalid buffers. */
		if (!(buf->flags & BUFFER_VALID))
		{
			blkunlock(buf);
			continue;
		}
		
		/* Prevent double free. */
		buf->count++;
		
		bdev_writeblk(buf);
	}
}

/*
 * Initializes the block buffer cache.
 */
PUBLIC void binit(void)
{
	int i;     /* Loop index.          */
	char *ptr; /* Buffer data pointer. */
	
	kprintf("fs: initializing the block buffer cache");
	
	/* Initialize block buffers. */
	ptr = (char *)BUFFERS_VIRT;
	for (i = 0; i < NR_BUFFERS; i++)
	{
		buffers[i].dev = 0;
		buffers[i].num = 0;
		buffers[i].data = ptr;
		buffers[i].count = 0;
		buffers[i].flags = ~(BUFFER_VALID | BUFFER_LOCKED | BUFFER_DIRTY);
		buffers[i].chain = NULL;
		buffers[i].free_next = 
			(i + 1 == NR_BUFFERS) ? &buffers[0] : &buffers[i + 1];
		buffers[i].free_prev = 
			(i - 1 < 0) ? &buffers[NR_BUFFERS - 1] : &buffers[i - 1];
		buffers[i].hash_next = &buffers[i];
		buffers[i].hash_prev = &buffers[i];
		
		ptr += BLOCK_SIZE;
	}
	
	/* Initialize the buffer cache. */
	free_buffers = &buffers[0];
	for (i = 0; i < BUFFERS_HASHTAB_SIZE; i++)
	{
		hashtab[i].hash_prev = &hashtab[i];
		hashtab[i].hash_next = &hashtab[i];
	}
}
