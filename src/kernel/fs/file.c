/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *              2017-2017 Romane Gallier <romanegallier@gmail.com>
 * 
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

#include <nanvix/clock.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <sys/types.h>
#include <dirent.h>
#include <errno.h>
#include "fs.h"
#include "minix/minix.h"

/*
 * Removes an entry from a directory.
 */
PUBLIC int dir_remove(struct inode *dinode, const char *filename)
{
	/* Check if the operation is valid */
	if (!dinode || !dinode->i_op || !dinode->i_op->dir_remove)
		return 0;
	return dinode->i_op->dir_remove(dinode, filename);
}

/*
 * Adds an entry to a directory.
 */
PUBLIC int dir_add(struct inode *dinode, struct inode *inode, const char *name)
{
	/* Check if the operation is valid */
	if (!dinode || !dinode->i_op || !dinode->i_op->dir_add)
		return 0;
	return dinode->i_op->dir_add(dinode, inode, name);
}

/*
 * Reads from a regular file.
 */
PUBLIC ssize_t file_read(struct inode *i, void *buf, size_t n, off_t off)
{
	/* Check if the operation is valid */
	if (!i || !i->i_op || !i->i_op->file_read)
		return 0;
	inode_lock(i);
	int retour =i->i_op->file_read(i,buf,n,off);
	inode_touch(i);
	inode_unlock(i);
	return retour;
}

/*
 * Reads from a regular directory.
 */
PUBLIC ssize_t dir_read(struct inode *i, void *buf, size_t n, off_t off)
{	
	inode_lock(i);

	if (i->flags & INODE_MOUNT)
		i=cross_mount_point_up(i);

	/* Check if the operation is valid */
	if (!i || !i->i_op || !i->i_op->dir_read){
		inode_unlock(i);
		return 0;
	}
	int retour = i->i_op->dir_read(i,buf,n,off);

	inode_touch(i);
	inode_unlock(i);
	return retour;
}

/*
 * Writes to a regular file.
 */
PUBLIC ssize_t file_write(struct inode *i, const void *buf, size_t n, off_t off)
{
	/* Check if the operation is valid */
	if (!i || !i->i_op || !i->i_op->file_read)
		return 0;
	inode_lock(i);
	int retour = i->i_op->file_write(i,buf,n,off);
	inode_touch(i);
	inode_unlock(i);
	return retour;
}

PUBLIC ino_t dir_search(struct inode *ip, const char *filename)
{
	struct buffer *buf; /* Block buffer.    */
	struct d_dirent *d; /* Directory entry. */
	int i;

	/* Cross mount point*/
	if ((ip->flags & INODE_MOUNT) && (kstrcmp (filename,"..")) )
	{
		ip = cross_mount_point_up(ip);
		i = 1;
	}

	else if ((root_fs(ip) == 1) && (!kstrcmp (filename,"..")))
	{
		ip = cross_mount_point_down(ip);
		i = 1;
	}
	
	/* Search directory entry. */
	d = ip->i_op->dirent_search(ip,filename, &buf, 0);

	if (d == NULL)
		return (INODE_NULL);
	
	brelse(buf);
	if (i == 1)
		inode_unlock(ip);
	
	return (d->d_ino);
}