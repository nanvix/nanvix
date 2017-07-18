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

#include <stdio.h>
#include <stdlib.h>

#include <sys/mount.h>

/**
 * @brief Prints program usage and exits.
 */
static void usage(void)
{
	printf("Usage: mkfs <input file> <file sytem name> <ninodes> <nblocks> \n");
	exit(EXIT_SUCCESS);
}

/**
 * @brief Creates a file system.
 */
int main(int argc, char **argv)
{
	unsigned ninodes;		/* # inodes in the file system.      */
	unsigned nblocks;		/* # data blocks in the file system. */
	const char *diskfile;	/* Disk file name.                   */
	const char *fs_name;	/* File system fs_name				 */	
	
	/* Missing arguments. */
	if (argc < 5)
		usage();
	
	/* Extract arguments. */
	diskfile = argv[1];
	fs_name= argv[2];
	sscanf(argv[3], "%u", &ninodes);
	sscanf(argv[4], "%u", &nblocks);

	printf("fs: mkfs %s,%s,%d,%d\n",diskfile,fs_name,ninodes,nblocks);

	int size = ninodes <<16 ;
	size = size +nblocks;

	mkfs(diskfile,fs_name, size);

	return (EXIT_SUCCESS);
}
