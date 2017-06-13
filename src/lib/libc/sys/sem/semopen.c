/*
 * Copyright(C) 2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2017 Davidson Francis <davidsondfgl@gmail.com>
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

#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <reent.h>
#include <nanvix/syscall.h>
#include <stdlib.h>

/**
 * @brief Performs operations in a semaphore.
 */

sem_t* sem_open(const char* name, int oflag, ...)
{	
	int ret;  /* Return value.		*/
	mode_t mode; /* Creation mode.		*/
	int value;	 /* semaphore value 	*/
	va_list arg; /* Variable argument. 	*/
	
	value = 0;
	mode = 0;

	if (oflag & O_CREAT)
	{
		va_start(arg, oflag);
		mode = va_arg(arg, mode_t);
		value = va_arg(arg,int);
		va_end(arg);
	}

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semopen),
		  "b" (name),
		  "c" (oflag),
		  "d" (mode),
		  "D" (value)
	);

	if (ret == (-1))
		return (NULL);

	sem_t* s;
	s = malloc(sizeof(sem_t));
	if (s == NULL)
	{
		errno = ENOMEM;
		return (NULL);
	}

	s->idx=ret;
	return (s);
}
