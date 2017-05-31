/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Davidson Francis <davidsondfgl@gmail.com>
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

#include <nanvix/syscall.h>
#include <errno.h>
#include <fcntl.h>
#include <stdarg.h>
#include <reent.h>
//#include <sys/sem.h>

/**
 * @brief Performs operations in a semaphore.
 */

int sem_close(sem_t* sem)
{	
	int ret;

	/* system call -> decrementing
	 * and deallocating if needed (0 proc using the semaphore)
	 */
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_semclose),
		  "b" (sem->idx)
	);

	/* making the semaphore unavailable for
	 * the calling process
	 */
	sem->idx=-1;
	
	return (ret);
}