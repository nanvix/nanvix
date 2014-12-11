/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

#include "minix.h"
#include "safe.h"

void minix_inode_write(uint16_t num, struct d_inode *ip);
const char *break_path(const char *pathname, char *filename);
uint16_t dir_search(struct d_inode *ip, const char *filename);
void minix_mount(const char *filename);
void minix_umount(void);
struct d_inode *minix_inode_read(uint16_t num);
uint16_t minix_mkdir(struct d_inode *ip, const char *filename);

/**
 * @brief Prints program usage and exits.
 * 
 * @details Prints program usage and exits.
 */
static void usage(void)
{
	printf("usage: minix.mkdir <input file> <directory>\n");
	exit(EXIT_SUCCESS);
}

/**
 * @brief Creates a directory in a Minix file system.
 */
int main(int argc, char **argv)
{
	const char *dirname;               /* Directory that shall be created. */
	uint16_t num1, num2;               /* Working inode numbers.           */
	struct d_inode *ip;                /* Working inode.                   */
	char filename[MINIX_NAME_MAX + 1]; /* Working file name.               */
	
	/* Wrong usage. */
	if (argc != 3)
		usage();

	minix_mount(argv[1]);
	
	dirname = argv[2];
	
	ip = minix_inode_read(num1 = 0);
	
	do
	{
		/* Break path. */
		dirname = break_path(dirname, filename);
		if (dirname == NULL)
		{
			fprintf(stderr, "minis.mkdir: bad pathname\n");
			goto out;
		}
		
		num2 = dir_search(ip, filename);
		
		/* Create directory. */
		if (num2 == INODE_NULL)
		{
			num2 = minix_mkdir(ip, filename);
		
			if (num2 == INODE_NULL)
				goto out;
		}
		
		minix_inode_write(num1, ip);
		ip = minix_inode_read(num1 = num2);
		
	} while (*filename != '\0');

out:

	minix_umount();
	
	return (EXIT_SUCCESS);
}
