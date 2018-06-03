/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2017 Davidson Francis <davidsondfgl@hotmail.com>
 *              2017-2018 Guillaume Besnard <guillaume.besnard@protonmail.com>
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

/**
 * @file nanvix/thread.h
 *
 * @brief Thread management system
 */

#ifndef NANVIX_THREAD_H_
#define NANVIX_THREAD_H_

	#include <nanvix/pm.h>
	#include <i386/fpu.h>
	#include <i386/pmc.h>
	#include <sys/types.h>


	#define THRD_KESP       0 /**< Kernel stack pointer offset.   */
	#define THRD_KSTACK     4 /**< Kernel stack pointer offset.   */
	#define THRD_FSS        8 /**< FPU Saved Status offset.       */

	/**
	 * @name Thread states
	 */
	/**@{*/
	#define THRD_DEAD       0 /**< Dead.             */
	#define THRD_ZOMBIE     1 /**< Zombie.           */
	#define THRD_RUNNING    2 /**< Ready to execute. */
	#define THRD_READY      3 /**< Running           */
	/**@}*/

	/**
	* @name Important system processes 
	*/
	/**@{*/
	#define THRD_IDLE (&threadtab[0]) /**< idle process. */
	/**@}*/


	/**
	 * @name Thread table boundaries
	 */
	/**@{*/
	#define FIRST_THRD ((&threadtab[0]))            /**< First process. */
	#define LAST_THRD ((&threadtab[THRD_MAX - 1])) 	/**< Last process.  */
	/**@}*/

	
#ifndef _ASM_FILE_

	/**
	 * @brief Thread.
	 */
	struct thread
	{
		/**
		 * @name Hard-coded Fields
		 */
		/**@{*/
		dword_t kesp;		/**< Kernel stack pointer.   */
		void *kstack;		/**< Kernel stack pointer.   */
		struct fpu fss;     /**< FPU Saved Status.       */
		struct pmc pmcs;    /**< PMC status.             */
		/**@}*/

		/**
		 * @name General information
		 */
		/**@{*/
		tid_t tid;          /**< Thread ID.              */
		/**@}*/

		/**
		 * @name Scheduling information
		 */
		/**@{*/
		unsigned state;         /**< Current state.                   */
		struct thread *next;    /**< Next threads owned by same proc. */
		/**@}*/
	};

	/* Forward definitions. */
	EXTERN struct process *thrd_father(struct thread *);

	/* Forward definitions. */
	EXTERN struct thread threadtab[THRD_MAX];
	EXTERN struct thread *curr_thread;
	EXTERN tid_t next_tid;

#endif /* _ASM_FILE */

#endif /* NANVIX_THREAD_H_ */
