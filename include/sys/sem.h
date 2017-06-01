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

#include <limits.h>
#include <sys/types.h>
#include <nanvix/const.h>

#ifndef _ASM_FILE_

#define SEM_IS_VALID(idx) \
		(idx>=0 && idx<MAX_SEM_NAME)

#define SEM_IS_FREE(idx) \
		(semtable[idx].nbproc=-1)

	/* kernel semaphore */
	typedef struct ksem {
		int value;                   /* value of the semaphore                    */
		char name[MAX_SEM_NAME];    /* name of the named semaphore               */
		int mode;                    /* permissions                               */
		int nbproc;                  /* number of processes sharing the semaphore */
		int unlinked;
	} ksem;

	/* Forward definitions. */
	extern ksem semtable[SEM_OPEN_MAX];

	/* Frees a semaphore */
	extern int freesem(int idx);

#endif