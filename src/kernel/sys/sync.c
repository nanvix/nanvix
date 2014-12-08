/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/sync.c - sync() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>

/*
 * Schedules file system updates.
 */
PUBLIC void sys_sync(void)
{
	inode_sync();
	superblock_sync();
	
	/*
	 * This will cause all dirty buffers
	 * to be written synchronously to disk.
	 */
	bsync();
}
