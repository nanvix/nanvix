/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/fs.c - File system manager.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/pm.h>
#include "fs.h"

/*
 * Root device.
 */
PUBLIC struct superblock *rootdev = NULL;
	
/*
 * Root directory.
 */
PUBLIC struct inode *root = NULL;

/*
 * Initializes the file system manager.
 */
PUBLIC void fs_init(void)
{
	cache_init();
	inode_init();
	superblock_init();
	
	rootdev = superblock_read(ROOT_DEV);
	
	/* Failed to read root super block. */
	if (rootdev == NULL)
		kpanic("failed to mount root file system");
	
	superblock_unlock(rootdev);
	
	root = inode_get(ROOT_DEV, 1);
	
	/* Failed to read root inode. */
	if (root == NULL)
		kpanic("failed to read root inode");
	
	kprintf("fs: root file system mounted");
	
	curr_proc->pwd = root;
	curr_proc->root = root;
}
