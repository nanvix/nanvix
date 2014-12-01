/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/stat.c - ustat() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <sys/types.h>
#include <ustat.h>

/*
 * Internal ustat().
 */
PRIVATE void do_ustat(dev_t dev, struct ustat *ubuf)
{
	int i, j;              /* Loop indexes.      */
	int tfree;             /* Total free blocks. */
	int tinode;            /* Total free inodes. */
	int bmap_size;         /* Size of block map. */
	int imap_size;         /* Size of inode map. */
	struct superblock *sb; /* Super block.       */
	
	sb = superblock_get(dev);
	
	/* Not mounted file system. */
	if (sb == NULL)
		return;
	
	/* Count number of free blocks. */
	tfree = 0;
	bmap_size = sb->zmap_blocks;
	for (i = 0; i < bmap_size; i++)
	{
		for (j = 0; j < (BLOCK_SIZE >> 2); j++)
		{
			tfree += bitmap_nclear(sb->zmap[i]->data, BLOCK_SIZE);
		}
	}
	
	/* Count number of free inodes. */
	tinode = 0;
	imap_size = sb->zmap_blocks;
	for (i = 0; i < imap_size; i++)
	{
		for (j = 0; j < (BLOCK_SIZE >> 2); j++)
		{
			tinode += bitmap_nclear(sb->imap[i]->data, BLOCK_SIZE);
		}
	}
	
	ubuf->f_tfree = tfree;
	ubuf->f_tinode = tinode;
	ubuf->f_fname[0] = '\0';
	ubuf->f_fpack[0] = '\0';
	
	superblock_put(sb);
}

/*
 * Gets file system statistics.
 */
PUBLIC int sys_ustat(dev_t dev, struct ustat *ubuf)
{
	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;

	/* Valid buffer. */
	if (chkmem(ubuf, sizeof(struct ustat), MAY_WRITE))
		do_ustat(dev, ubuf);

	return (curr_proc->errno);
}
