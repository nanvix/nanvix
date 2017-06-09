/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef SEMAPHORE_H_
#define SEMAPHORE_H_

#ifndef _ASM_FILE_

	/**
	 * @brief User-land semaphore
	 *
	 */
	typedef struct sem_t
	{
		int idx; /**< Semaphore ID.  */
	} sem_t;

	/* Forward definitions. */
	extern sem_t *sem_open(const char *, int, ...);
	extern int sem_close(sem_t *);
	extern int sem_unlink(const char *);
	extern int sem_post(sem_t *);
	extern int sem_wait(sem_t *);

#endif

#endif /* SEMAPHORE_H_ */
