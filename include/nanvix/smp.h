/*
 * Copyright(C) 2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef SMP_H_
#define SMP_H_

	#include <nanvix/const.h>

	/* IPI Message types. */
	#define IPI_WAKEUP      0x01
	#define IPI_SCHEDULE    0x02
	#define IPI_SYSCALL     0x04
	#define IPI_INTERRUPT   0x08
	#define IPI_EXCEPTION   0x10
	#define IPI_VFAULT      0x20
	#define IPI_PFAULT      0x40

	/* per_core structure. */
	struct per_core
	{
		unsigned coreid;
		unsigned syscallno;
		struct intstack *ints;
		struct process *curr_proc;
		//struct thread *curr_thread;
	};

	/* External functions. */
	EXTERN unsigned smp_get_coreid(void);
	EXTERN unsigned smp_get_numcores(void);
	EXTERN void smp_init(void);
	
	/* Spinlock primitives. */
	EXTERN void spin_init(spinlock_t *);
	EXTERN void spin_lock(spinlock_t *);
	EXTERN void spin_unlock(spinlock_t *);

	/* External variable. */
	EXTERN unsigned smp_enabled;
	EXTERN unsigned release_cpu;
	EXTERN spinlock_t boot_lock;
	EXTERN unsigned release_ipi;
	EXTERN spinlock_t ipi_lock;
	EXTERN struct per_core cpus[NR_CPUS];

#endif /* SMP_H_ */
