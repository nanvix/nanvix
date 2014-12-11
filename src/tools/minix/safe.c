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

#include <sys/types.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>

/**
 * @brief Safe open().
 */
int sopen(const char *pathname, int flags)
{
	int fd;
	
	fd = open(pathname, flags);
	if (fd == -1)
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
	
	return (fd);
}

/**
 * @brief Safe close().
 */
void sclose(int fd)
{
	int ret;
	
	ret = close(fd);
	if (ret == -1)
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe lseek().
 */
void slseek(int fd, off_t offset, int whence)
{
	off_t ret;
	
	ret = lseek(fd, offset, whence);
	if (ret == (-1))
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe read().
 */
void sread(int fd, void *buf, size_t count)
{
	ssize_t ret;
	
	ret = read(fd, buf, count);
	if (ret != count)
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe write().
 */
void swrite(int fd, const void *buf, size_t count)
{
	ssize_t ret;
	
	ret = write(fd, buf, count);
	if (ret != count)
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
}

/**
 * @brief Safe malloc();
 */
void *smalloc(size_t n)
{
	void *p;
	
	p = malloc(n);
	if (p == NULL)
	{
		perror(NULL);
		exit(EXIT_FAILURE);
	}
	
	return (p);
}
