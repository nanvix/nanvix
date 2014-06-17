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
	bsync();
	inode_sync();
	superblock_sync();
}
