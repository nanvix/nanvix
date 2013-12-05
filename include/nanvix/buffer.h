/*
 * Copyright(C) 2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <nanvix/buffer.h> - Buffer cache library.
 */

#ifndef BUFFER_H_
#define BUFFER_H_

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	#include <sys/types.h>

	/* Buffer size. */
	#define BUFFER_SIZE 1024
	
	/*
	 * Buffer.
	 */
	struct buffer
	{
		/* General information. */
		dev_t dev;             /* Device.          */
		unsigned num;          /* Block number.    */
		int locked;            /* Locked?          */
		int dirty;             /* Dirty?           */
		int valid;             /* Valid data?      */
		struct process *chain; /* Sleeping chain.  */
		void *data;            /* Underlying data. */
		
		/* Buffer cache information. */
		struct buffer *free_next; /* Next buffer in the free list.      */
		struct buffer *free_prev; /* Previous buffer in the free list.  */
		struct buffer *hash_next; /* Next buffer in the hash table.     */
		struct buffer *hash_prev; /* Previous buffer in the hash table. */
	};
	
	/*
	 * Initializes the buffer cache.
	 */
	EXTERN void buffer_init(void);
	
	/*
	 * Gets a buffer.
	 */
	EXTERN struct buffer *getblk(dev_t dev, unsigned block);
	
	/*
	 * Releases a buffer.
	 */
	EXTERN void brelse(struct buffer *buf);
	
	/*
	 * Reads a block.
	 */
	EXTERN struct buffer *bread(dev_t dev, unsigned block);
	
	/*
	 * Writes a block.
	 */
	EXTERN int bwrite(struct buffer *buf);

#endif /* BUFFER_H_ */
