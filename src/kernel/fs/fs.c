/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * This file is part of Nanvix.
 *
 * Nanvix is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * Nanvix is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/mm.h>
#include <nanvix/pm.h>
#include <dirent.h>
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
 * Gets an empty file descriptor table entry.
 */
PUBLIC int getfildes(void)
{
	int i;

	/* Look for empty file descriptor table entry. */
	for (i = 0; i < OPEN_MAX; i++)
	{
		/* Found. */
		if (curr_proc->ofiles[i] == NULL)
			return (i);
	}

	return (-1);
}

/*
 * Gets an empty file table entry.
 */
PUBLIC struct file *getfile(void)
{
	struct file *f;

	/* Look for empty file table entry. */
	for (f = &filetab[0]; f < &filetab[NR_FILES]; f++)
	{
		/* Found. */
		if (f->count == 0)
			return (f);
	}

	return (NULL);
}

/*
 * Closes a file.
 */
PUBLIC void do_close(int fd)
{
	struct inode *i; /* Inode. */
	struct file *f;  /* File.  */

	f = curr_proc->ofiles[fd];

	/* Not opened. */
	if (f == NULL)
		return;

	curr_proc->close &= ~(1 << fd);
	curr_proc->ofiles[fd] = NULL;

	if (--f->count)
		return;

	inode_lock(i = f->inode);
	inode_put(i);
}

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
		mode &= S_IRWXU | S_IRWXG | S_IRWXO;

	/* Owner's group user. */
	else if ((proc->gid == gid) || ((!oreal && proc->egid == gid)))
		mode &= S_IRWXG | S_IRWXO;

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
	if ((kname = getkpg(0)) == NULL)
	{
		curr_proc->errno = -ENOMEM;
		return (NULL);
	}

	/* Copy user file name. */
	for (r = name, w = kname; (ch = fubyte(r)) != '\0'; r++, w++)
	{
		/* Bad user file name. */
		if (ch < 0)
		{
			putkpg(kname);
			curr_proc->errno = -EFAULT;
			return (NULL);

		}

		/* File name too long. */
		if ((w - kname) > PAGE_SIZE - 1)
		{
			putkpg(kname);
			curr_proc->errno = -ENAMETOOLONG;
			return (NULL);
		}

		*w = ch;
	}

	*w = '\0';

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
	binit();
	inode_init();
	superblock_init();

	/* Sanity check. */
	CHKSIZE(sizeof(struct d_dirent), sizeof(struct dirent));

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

	/* Hand craft idle process. */
	IDLE->pwd = root;
	IDLE->root = root;
	root->count += 2;

	inode_unlock(root);
}
