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
#include <nanvix/dev.h>
#include <nanvix/fs.h>
#include <errno.h>
#include <fcntl.h>
#include <limits.h>
#include <stdarg.h>

/*
 * Returns access permissions.
 */
#define PERM(o)                                        \
	((ACCMODE(o) == O_RDWR) ? (MAY_READ | MAY_WRITE) : \
	((ACCMODE(o) == O_WRONLY) ? MAY_WRITE :            \
	(MAY_READ | ((o & O_TRUNC) ? MAY_WRITE : 0))))     \

/*
 * Creates a file.
 */
PRIVATE struct inode *do_creat(struct inode *d, const char *name, mode_t mode, int oflag)
{
	struct inode *i;

	/* Not asked to create file. */
	if (!(oflag & O_CREAT))
	{
		curr_proc->errno = -ENOENT;
		return (NULL);
	}

	/* Not allowed to write in parent directory. */
	if (!permission(d->mode, d->uid, d->gid, curr_proc, MAY_WRITE, 0))
	{
		curr_proc->errno = -EACCES;
		return (NULL);
	}

	i = inode_alloc(d->sb);

	/* Failed to allocate inode. */
	if (i == NULL)
		return (NULL);

	i->mode = (mode & MAY_ALL & ~curr_proc->umask) | S_IFREG;

	/* Failed to add directory entry. */
	if (dir_add(d, i, name))
	{
		inode_put(i);
		return (NULL);
	}

	inode_unlock(i);

	return (i);
}

/*
 * Opens a file.
 */
PRIVATE struct inode *do_open(const char *path, int oflag, mode_t mode)
{
	int err;              /* Error?               */
	const char *name;     /* File name.           */
	struct inode *dinode; /* Directory's inode.   */
	ino_t num;            /* File's inode number. */
	dev_t dev;            /* File's device.       */
	struct inode *i;      /* File's inode.        */

	dinode = inode_dname(path, &name);

	/* Failed to get directory. */
	if (dinode == NULL)
		return (NULL);

	num = dir_search(dinode, name);

	/* File does not exist. */
	if (num == INODE_NULL)
	{
		i = do_creat(dinode, name, mode, oflag);

		/* Failed to create inode. */
		if (i == NULL)
		{
			inode_put(dinode);
			return (NULL);
		}

		inode_put(dinode);
		return (i);
	}

	dev = dinode->dev;
	inode_put(dinode);

	/* File already exists. */
	if (oflag & O_EXCL)
	{
		curr_proc->errno = -EEXIST;
		return (NULL);
	}

	i = inode_get(dev, num);

	/* Failed to get inode. */
	if (i == NULL)
		return (NULL);

	/* Not allowed. */
	if (!permission(i->mode, i->uid, i->gid, curr_proc, PERM(oflag), 0))
	{
		curr_proc->errno = -EACCES;
		goto error;
	}

	/* Character special file. */
	if (S_ISCHR(i->mode))
	{
		err = cdev_open(i->blocks[0]);

		/* Failed to open character device. */
		if (err)
		{
			curr_proc->errno = err;
			goto error;
		}
	}

	/* Block special file. */
	else if (S_ISBLK(i->mode))
	{
		/* TODO: open device? */
	}

	/* Pipe file. */
	else if (S_ISFIFO(i->mode))
	{
		curr_proc->errno = -ENOTSUP;
		goto error;
	}

	/* Regular file. */
	else if (S_ISREG(i->mode))
	{
		/* Truncate file. */
		if (oflag & O_TRUNC)
			inode_truncate(i);
	}

	/* Directory. */
	else if (S_ISDIR(i->mode))
	{
		/* Directories are not writable. */
		if (ACCMODE(oflag) != O_RDONLY)
		{
			curr_proc->errno = -EISDIR;
			goto error;
		}
	}

	inode_unlock(i);

	return (i);

error:
	inode_put(i);
	return (NULL);

}

/*
 * Opens a file.
 */
PUBLIC int sys_open(const char *path, int oflag, mode_t mode)
{
	int fd;           /* File descriptor.  */
	struct file *f;   /* File.             */
	struct inode *i;  /* Underlying inode. */
	char *name;       /* Path name.        */

	/* Fetch path from user address space. */
	if ((name = getname(path)) == NULL)
		return (curr_proc->errno);

	fd = getfildes();

	/* Too many opened files. */
	if (fd >= OPEN_MAX)
	{
		putname(name);
		return (-EMFILE);
	}

	f = getfile();

	/* Too many files open in the system. */
	if (f == NULL)
	{
		putname(name);
		return (-ENFILE);
	}

	/* Increment reference count before actually opening
	 * the file because we can sleep below and another process
	 * may want to use this file table entry also.  */
	f->count = 1;

	/* Open file. */
	if ((i = do_open(name, oflag, mode)) == NULL)
	{
		putname(name);
		f->count = 0;
		return (curr_proc->errno);
	}

	/* Initialize file. */
	f->oflag = oflag;
	f->pos = 0;
	f->inode = i;

	curr_proc->ofiles[fd] = f;
	curr_proc->close &= ~(1 << fd);

	putname(name);

	return (fd);
}
