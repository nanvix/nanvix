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

	#include <nanvix/config.h>
	#include <nanvix/pm.h>
	#include <i386/fpu.h>
	#include <i386/pmc.h>
	#include <sys/types.h>
	#include <nanvix/region.h>

	/**
	 * @name Offsets to hard-coded fields of a process
	 */
	/**@{*/
	#define THRD_KESP         0 /**< Kernel stack pointer offset.   */
	#define THRD_KSTACK       4 /**< Kernel stack pointer offset.   */
	#define THRD_FLAGS        8 /**< Thread flags offset.           */
	#define THRD_INTSTACK    12 /**< Interrupt Stack.               */
	#define THRD_IPISTACK    16 /**< IPI Kernel stack.              */
	#define THRD_FSS         20 /**< FPU Saved Status offset.       */
	#define THRD_PMC        128 /**< Performance Counter Status.    */
	/**@}*/

	/**
	 * @name Thread types
	 */
	/**@{*/
	#define THRD_MAIN 0 /**< Is it the main thread? */
	/**@}*/

	/**
	 * @name Thread flags
	 */
	/**@{*/
	#define THRD_NEW 0 /**< Is the thread new?      */
	#define THRD_SYS 1 /**< Handling a system call? */
	/**@}*/

	/**
	 * @name Thread states
	 */
	/**@{*/
	#define THRD_DEAD       0 /**< Dead.             */
	#define THRD_READY      1 /**< Ready to execute. */
	#define THRD_RUNNING    2 /**< Running.          */
	#define THRD_STOPPED    3 /**< Stopped.          */
	#define THRD_WAITING    4 /**< Waiting.          */
	#define THRD_TERMINATED 5 /**< Terminated.       */
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
	#define FIRST_THRD ((&threadtab[1]))            /**< First process. */
	#define LAST_THRD  ((&threadtab[THRD_MAX - 1])) /**< Last process.  */
	/**@}*/


#ifndef _ASM_FILE_

	/**
	 * @name Thread configuration.
	 */
	/**@{*/
	#define THRD_STACK_SIZE PGTAB_SIZE /**< Thread stack size. */

	#if THRD_MAX_PER_PROC > THRD_MAX
        #error "THRD_MAX_PER_PROC should not exceed THRD_MAX"
    #elif THRD_MAX_PER_PROC > REGION_SIZE_CPP/THRD_STACK_SIZE
        #error "THRD_MAX_PER_PROC should not exceed REGION_SIZE/THRD_STACK_SIZE"
    #elif THRD_MAX > (REGION_SIZE_CPP/THRD_STACK_SIZE*PROC_MAX)
        #error "THRD_MAX should not exceed REGION_SIZE/THRD_STACK_SIZE*PROC_MAX"
    #endif
	/**@}*/

	/**
	 * @brief Thread.
	 */
	struct thread
	{
		/**
		 * @name Hard-coded Fields
		 */
		/**@{*/
		dword_t kesp;          /**< Kernel stack pointer.   */
		void *kstack;          /**< Kernel stack pointer.   */
		unsigned flags;        /**< Thread flags.           */
		struct intstack *ints; /**< Interrupt Stack.        */
		void *ipikstack;       /*<< IPI Kernel stack.       */
		struct fpu fss;        /**< FPU Saved Status.       */
		struct pmc pmcs;       /**< PMC status.             */
		/**@}*/

		/**
		 * @name Memory information
		 */
		/**@{*/
		/* TODO : keep in process memory regions,
		 * but divide the space between threads   */
		struct pregion pregs; /**< Thread stack memory regions. */
		/**@}*/

		/**
		 * @name General information
		 */
		/**@{*/
		tid_t tid;          /**< Thread ID.              */
		void *retval;       /**< Thread return value.    */
		/**@}*/

		/**
		 * @name Scheduling information
		 */
		/**@{*/
		unsigned state;         /**< Current state.                   */
		int counter;            /**< Remaining quantum.               */
		struct thread *next;    /**< Next threads owned by same proc. */
		/**@}*/
	};

	/* Forward definitions. */
	EXTERN void thread_init(void);
	EXTERN struct process *thrd_father(struct thread *);
	EXTERN struct thread *get_free_thread();
	EXTERN int clear_thread(struct thread *thrd);

	/* Forward definitions. */
	EXTERN struct thread threadtab[THRD_MAX];
	EXTERN tid_t next_tid;

#endif /* _ASM_FILE */

#endif /* NANVIX_THREAD_H_ */
