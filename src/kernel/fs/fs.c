/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * fs/fs.c - File system manager.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/paging.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <errno.h>
#include "fs.h"

#if NR_FILES < OPEN_MAX*NR_PROCS
	#error "file table too big"
#endif

/*
 * Root device.
 */
PUBLIC struct superblock *rootdev = NULL;
	
/*
 * Root directory.
 */
PUBLIC struct inode *root = NULL;

/*
 * File table.
 */
PUBLIC struct file filetab[NR_FILES];

/*
 * Checks rwx permissions on a file.
 */
PUBLIC mode_t permission(mode_t mode, uid_t uid, gid_t gid, struct process *proc, mode_t mask, int oreal)
{
	mode &= mask;
	
	/* Super user. */
	if (IS_SUPERUSER(proc))
		mode &= S_IRWXU | S_IRWXG | S_IRWXO;
	
	/* Owner user. */
	else if ((proc->uid == uid) || ((!oreal && proc->euid == uid)))
		mode &= S_IRWXU;
	
	/* Owner's group user. */
	else if ((proc->gid == gid) || ((!oreal && proc->egid == gid)))
		mode &= S_IRWXG;
	
	/* Other user. */
	else
		mode &= S_IRWXO;
	
	return (mode);
}

/*
 * Gets a user file name
 */
PUBLIC char *getname(const char *name)
{
	int ch;        /* Temporary character. */
	char *kname;   /* Kernel user name.    */
	const char *r; /* Read pointer.        */
	char *w;       /* Write pointer.       */
	
	/* Grab a kernel page. */
	if ((kname = getkpg()) == NULL)
	{
		curr_proc->errno = -ENOMEM;
		return (NULL);
	}
	
	/* Copy user file name. */
	for (r = name, w = kname; (ch = fubyte(name)) != '\0'; r++, w++)
	{
		/* Bad user file name. */
		if (ch < 0)
		{
			putkpg(kname);
			curr_proc->errno = -EFAULT;
			return (NULL);
			
		}
		
		/* File name too long. */
		if ((w - kname) > PAGE_SIZE)
		{
			putkpg(kname);
			curr_proc->errno = -ENAMETOOLONG;
			return (NULL);
		}
		
		*w = ch;
	}
	
	return (kname);
}

/*
 * Puts back a user file name.
 */
PUBLIC void putname(char *name)
{
	putkpg(name);
}

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
	
	inode_unlock(root);
}
