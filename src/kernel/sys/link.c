/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * sys/link.c - link() system call implementation.
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <errno.h>
#include <limits.h>
#include <unistd.h>

/*
 * Internal link().
 */
PRIVATE void do_link(const char *source, const char *target)
{
	const char *name;      /* File name.                                */
	struct inode *isource; /* inode of source file.                     */
	struct inode *iparent; /* inode of parent directory of target file. */
	
	isource = inode_name(source);
	
	/* Failed to get inode of source file. */
	if (isource == NULL)
		return;
		
	/* Unprivileged user linking directory. */
	if (S_ISDIR(isource->mode) && (!IS_SUPERUSER(curr_proc)))
	{
		curr_proc->errno = EPERM;
		inode_put(isource);
		return;
	}
	
	/* Too many links. */
	if (isource->nlinks == LINK_MAX)
	{
		curr_proc->errno = EMLINK;
		inode_put(isource);
		return;
	}
	
	isource->nlinks++;
	inode_unlock(isource);
	
	iparent = inode_dname(target, &name);
	
	/* Failed to get parent directory of target file. */
	if (iparent == NULL)
	{
		inode_lock(isource);
		isource->nlinks--;
		inode_put(isource);
		return;
	}
	
	/* Link target file to source file. */
	if (dir_add(iparent, isource, name) < 0)
	{
		inode_put(iparent);
		inode_lock(isource);
		isource->nlinks--;
		inode_put(isource);
	}
	
	inode_put(iparent);
	inode_lock(isource);
	inode_put(isource);	
}

/*
 * Links a name to a file.
 */
PUBLIC int sys_link(const char *path1, const char *path2)
{
	char *target; /* Target path. */
	char *source; /* Source path. */
	
	/*
	 * Reset errno, since we may
	 * use it in kernel routines.
	 */
	curr_proc->errno = 0;
	
	source = getname(path1);
	
	/* Failed to get source name. */
	if (source == NULL)
		goto out0;
	
	target = getname(path2);
	
	/* Failed to get target name. */
	if (target == NULL)
		goto out1;
	
	do_link(source, target);
	
	putname(target);
out1:
	putname(source);
out0:
	return (-curr_proc->errno);
}
