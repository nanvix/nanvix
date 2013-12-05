/*
 * Copyright(C) 2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/buffer.c - Buffer cache implementation.
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/dev.h>
#include <nanvix/buffer.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>

/* Number of buffers (don't touch). */
#define NR_BUFFER 256

/* Hash table size. */
#define HASHTAB_SIZE 127

/* Buffers. */
PRIVATE struct buffer buffers[NR_BUFFER];

/* Hash table. */
PRIVATE struct buffer *hashtab[HASHTAB_SIZE];

/* Free list. */
PRIVATE struct buffer *free;

/* Processes waiting any block. */
PRIVATE struct process *chain = NULL;

/*
 * Hash function.
 */
#define HASHFN(dev, block) (((dev)^(block))%HASHTAB_SIZE)

/*
 * Sleeps on a buffer.
 */
PRIVATE void buffer_sleep(struct process **chain)
{
	cpulvl(CPULVL_DISK);
	sleep(chain, PRIO_BUFFER);
	cpulvl(CPULVL_NONE);
}

/*
 * Inserts a buffer in the cache.
 */
PRIVATE void buffer_insert(struct buffer *buf)
{
	wakeup(&chain);
	
	cpulvl(CPULVL_DISK);
	
	wakeup(&buf->chain);
	
	/* Empty free list. */
	if (free == NULL)
		free = buf;
	
	else
	{	
		buf->free_next = free;
		buf->free_prev = free->free_prev;
				
		/* Frenquently used buffer (insert in the end). */
		if (buf->valid && !buf->dirty)
		{	
			free->free_prev->free_next = buf;
			free->free_prev = buf;
		}
		
		/* Not frequently used buffer (insert in the begin). */
		else
		{	
			free->free_next->free_prev = buf;
			free->free_prev = buf;	
			free = buf;
		}
	}
	
	buf->locked = 0;
	
	cpulvl(CPULVL_NONE);
}

/*
 * Updates a buffer in the cache.
 */
PRIVATE void buffer_update
(struct buffer *buf, dev_t dev, unsigned num, int valid)
{
	buf->dev = dev;
	buf->num = num;
	buf->valid = valid;
	
	buf->hash_next = hashtab[HASHFN(buf->dev, buf->num)];
	buf->hash_prev = NULL;
	hashtab[HASHFN(buf->dev, buf->num)] = buf;
	if (buf->hash_next != NULL)
		buf->hash_next->hash_prev = buf;
}

/*
 * Removes a buffer from the cache.
 */
PRIVATE struct buffer *buffer_remove(struct buffer *buf)
{
	/* Remove first buffer from the free list. */
	if (buf == NULL)
	{
		buf = free;
		
		/* No free buffers. */
		if (buf == NULL)
			return (NULL);
	
		/* Remove buffer from hash queue. */
		if (buf->hash_prev != NULL)
			buf->hash_prev->free_next = buf->hash_next;
		if (buf->hash_next != NULL)
			buf->hash_next->free_prev = buf->hash_prev;
		if (hashtab[HASHFN(buf->dev, buf->num)] == buf)
			hashtab[HASHFN(buf->dev, buf->num)] = buf->hash_next;
	}
	
	cpulvl(CPULVL_DISK);
	
	/* Remove buffer from the free list. */
	buf->free_prev->free_next = buf->free_next;
	buf->free_next->free_prev = buf->free_prev;
	if (buf->free_next == buf->free_prev)
		free = NULL;
	else if (free == buf)
		free = buf->free_next;
		
	buf->locked = 1;
	
	cpulvl(CPULVL_NONE);
	
	return (buf);
}

/*
 * Releases a buffer.
 */
PUBLIC void brelse(struct buffer *buf)
{
	buffer_insert(buf);
}

/*
 * Gets a buffer.
 */
PUBLIC struct buffer *getblk(dev_t dev, unsigned block)
{
	int err;
	struct buffer *buf;
	
repeat:

	/* Search in hash table. */
	for (buf = hashtab[HASHFN(dev, block)]; buf != NULL; buf = buf->hash_next)
	{
		/* Found. */
		if ((buf->dev == dev) && (buf->num == block))
			break;
	}
	
	/* Buffer is in the hash table.  */
	if (buf != NULL)
	{
		/* Buffer is locked. */
		if (buf->locked)
		{
			buffer_sleep(&buf->chain);
			goto repeat;
		}
		
		return (buffer_remove(buf));
	}

	buf = buffer_remove(NULL);

	/* There are no free buffers. */
	if (buf == NULL)
	{
		buffer_sleep(&chain);
		goto repeat;
	}
	
	/* Synchronize buffer. */
	if (buf->dirty)
	{
		buffer_update(buf, buf->dev, buf->num, buf->valid);
		err = bdev_writeblk(buf);
		if (err)
			buffer_insert(buf);
		goto repeat;
	}
	
	buffer_update(buf, dev, block, 0);

	return (buf);
}

/*
 * Reads a block.
 */
PUBLIC struct buffer *bread(dev_t dev, unsigned block)
{
	int err;
	struct buffer *buf;
	
	buf = getblk(dev, block);
	
	/* Valid buffer. */
	if (buf->valid)
		return (buf);
	
	err = bdev_readblk(buf);
	
	/* Failed to read block. */
	if (err)
	{
		brelse(buf);
		return (NULL);
	}
	
	return (buf);
}

/*
 * Writes a block.
 */
PUBLIC int bwrite(struct buffer *buf)
{
	brelse(buf);
	
	return (0);
}

/*
 * Initializes the buffer cache.
 */
PUBLIC void buffer_init(void)
{
	int i;
	char *ptr;
	
	ptr = (char *)BUFFERS_VIRT;
	
	/* Initialize buffers. */
	for (i = 0; i < NR_BUFFER; i++)
	{		
		buffers[i].dev = 0;
		buffers[i].num = 0;
		buffers[i].locked = 0;
		buffers[i].dirty = 0;
		buffers[i].valid = 0;
		buffers[i].chain = NULL;
		buffers[i].data = ptr;
		buffers[i].free_next = &buffers[(i + 1)%NR_BUFFER];
		buffers[i].free_prev = &buffers[(i - 1)%NR_BUFFER]; /* See below. */
		buffers[i].hash_next = NULL;
		buffers[i].hash_prev = NULL;
		
		ptr += BUFFER_SIZE;
	}
	
	/* Initialize buffer cache. */
	free = &buffers[0];
	free->free_prev = &buffers[NR_BUFFER - 1];
	for (i = 0; i < HASHTAB_SIZE; i++)
		hashtab[i] = NULL;
	
	kprintf("INIT: buffer cache initialized");
}
