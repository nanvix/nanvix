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
#include <errno.h>



/**
 * @brief Creates a Minix file system.
 */
PUBLIC int sys_mkfs(const char *diskfile, const char *fs_name, int size)
{
	unsigned ninodes;     	/* # inodes in the file system.     	*/
	unsigned nblocks;		/* # data blocks in the file system.	*/
	char * kdiskfile;		/* file of the diskfile 				*/
	char * kfs_name;		/* name of the file system				*/

	/*Get the number of inode*/
	ninodes=size >>16;
	
	/*Get the number of block*/
	nblocks=(size<<16)>>16;

	/* Get device name. */
	if ((kdiskfile = getname(diskfile)) == NULL)
		return (curr_proc->errno);
	
	/* Get target directory. */
	if ((kfs_name = getname(fs_name)) == NULL)
		return (curr_proc->errno);

	kprintf("%s,%s,%d,%d",kdiskfile,kfs_name, ninodes, nblocks);

	mkfs(diskfile, ninodes, nblocks,0,0);
	return 1;
}
