/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/super.c - Super block library implementation.
 */

#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/fs.h>
#include "fs.h"

/* Super block table. */
PUBLIC struct superblock superblocks[NR_SUPERBLOCKS];

/*
 * Gets an empty super block.
 */
PRIVATE struct superblock *superblock_empty(void)
{
	struct superblock *sb;
	
	/* Get empty super block. */
	for (sb = &superblocks[0]; sb < &superblocks[NR_SUPERBLOCKS]; sb++)
	{
		superblock_lock(sb);
		
		/* Found. */
		if (!(sb->flags & SUPERBLOCK_VALID))
		{
			sb->flags = SUPERBLOCK_VALID;
			superblock_lock(sb);
			return (sb);
		}
		
		superblock_unlock(sb);
	}

	/* Super block table */
	kprintf("fs: super block table overflow");
	
	return (NULL);
}

/*
 * Gets super block.
 */
PUBLIC struct superblock *superblock_get(dev_t dev)
{
	struct superblock *sb;
	
	/* Get empty super block. */
	for (sb = &superblocks[0]; sb < &superblocks[NR_SUPERBLOCKS]; sb++)
	{
		superblock_lock(sb);
		
		/* Valid. */
		if (sb->flags & SUPERBLOCK_VALID)
		{
			/* Found. */
			if (sb->dev == dev)
				return (sb);
		}
		
		superblock_unlock(sb);
	}
	
	return (NULL);
}

/*
 * Locks a super block.
 */
PUBLIC void superblock_lock(struct superblock *sb)
{
	/* Wait for super block to become unlocked. */
	while (sb->flags & SUPERBLOCK_LOCKED)
		sleep(&sb->chain, PRIO_SUPERBLOCK);
	
	sb->flags |= SUPERBLOCK_LOCKED;
}

/*
 * Unlocks a super block.
 */
PUBLIC void superblock_unlock(struct superblock *sb)
{
	wakeup(&sb->chain);
	sb->flags &= ~SUPERBLOCK_LOCKED;
}

/*
 * Releases a super block.
 */
PUBLIC void superblock_put(struct superblock *sb)
{
	int i;
	
	superblock_write(sb);
	
	/* Release inode map buffers. */
	for (i = 0; i < (int) sb->imap_blocks; i++)
		block_put(sb->imap[i]);
	
	/* Release zone map buffers. */
	for (i = 0; i < (int) sb->zmap_blocks; i++)
		block_put(sb->zmap[i]);
	
	/* Release super block buffer. */
	block_put(sb->buf);
	
	sb->flags &= ~SUPERBLOCK_VALID;
	
	superblock_unlock(sb);
}

/*
 * Writes a super block to a device.
 */
PUBLIC void superblock_write(struct superblock *sb)
{
	int i;                     /* Loop index.       */
	struct d_superblock *d_sb; /* Disk super block. */
	
	/* Nothing to be done. */
	if (!(sb->flags & SUPERBLOCK_DIRTY))
		return;
	
	/* Write inode map buffers. */
	for (i = 0; i < (int) sb->imap_blocks; i++)
		block_write(sb->imap[i]);
	
	/* Write zone map buffers. */
	for (i = 0; i < (int) sb->zmap_blocks; i++)
		block_write(sb->zmap[i]);
	
	/* Write super block buffer. */
	d_sb = (struct d_superblock *) sb->buf->data;
	d_sb->s_firstdatazone = sb->firstdatazone;
	sb->buf->flags |= BUFFER_DIRTY;
	block_write(sb->buf);
	
	sb->flags &= ~SUPERBLOCK_DIRTY;
}

/*
 * Reads a super block from a device.
 */
PUBLIC struct superblock *superblock_read(dev_t dev)
{
	int i;                     /* Loop indexes.            */
	struct buffer *buf;        /* Buffer disk super block. */
	struct superblock *sb;     /* In-core super block.     */
	struct d_superblock *d_sb; /* Disk super block.        */
		
	sb = superblock_empty();
	
	/* Super block table overflow. */
	if (sb == NULL)
		goto error0;
	
	buf = block_read(dev, 1);
	
	d_sb = (struct d_superblock *)buf->data;
	
	/* Bad magic number. */
	if (d_sb->s_magic != SUPER_MAGIC)
		goto error1;
	
	/* Do not mount weird filed systems. */
	if (d_sb->s_log_zone_size != 0)
	{
		kprintf("fs: file system on device %d has weird zone size", dev);
		goto error1;
	}
	
	/* Too many blocks in the inode/zone map. */
	if ((d_sb->s_imap_blocks > IMAP_SIZE) || (d_sb->s_zmap_blocks > ZMAP_SIZE))
	{
		kprintf("fs: too many blocks in the inode/zone map");
		goto error1;
	}
	
	/* Initialize super block. */
	sb->buf = buf;
	sb->ninodes = d_sb->s_ninodes;
	sb->imap_blocks = d_sb->s_imap_blocks;
	for (i = 0; i < (int) sb->imap_blocks; i++)
	{
		sb->imap[i] = block_read(dev, 2 + i);
		block_unlock(sb->imap[i]);
	}
	sb->zmap_blocks = d_sb->s_zmap_blocks;
	for (i = 0; i < (int) sb->zmap_blocks; i++)
	{
		sb->zmap[i] = block_read(dev, 2 + sb->imap_blocks + i);
		block_unlock(sb->zmap[i]);
	}
	sb->firstdatazone = d_sb->s_firstdatazone;
	sb->log_zone_size = d_sb->s_log_zone_size;
	sb->max_size = d_sb->s_max_size;
	sb->zones = d_sb->s_nzones;
	sb->root = NULL;
	sb->mp = NULL;
	sb->dev = dev;
	sb->flags &= ~(SUPERBLOCK_DIRTY | SUPERBLOCK_RDONLY);
	sb->flags |= SUPERBLOCK_VALID;
	sb->isearch = 0;
	sb->zsearch = 0;
	sb->chain = NULL;
	
	block_unlock(buf);
	
	return (sb);
	
error1:
	block_put(buf);
	superblock_put(sb);
error0:
	return (NULL);
}

/*
 * Initialize super blocks.
 */
PUBLIC void superblock_init(void)
{
	int i;
	
	/* Initialize super blocks. */
	for (i = 0; i < NR_SUPERBLOCKS; i++)
		superblocks[i].flags &= ~SUPERBLOCK_VALID;
	
	kprintf("fs: super block table has %d entries", NR_SUPERBLOCKS);
}
