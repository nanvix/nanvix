/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
 *              2017-2017 Davidson Francis <davidsondfgl@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <nanvix/mm.h>
#include <nanvix/clock.h>
#include <nanvix/debug.h>
#include <nanvix/smp.h>
#include <fcntl.h>

/* External declaration for cpu_init() function. */
extern void cpu_init(void);

/**
 * @brief Required function to use GCC varargs
 */
PUBLIC void abort()
{
}

/**
 * @brief Idle for UniProcessor systems.
 */
PUBLIC void idle_up(void)
{
	struct process *p; /* Working process.  */

	while (1)
	{
		/* Shutting down.*/
		if (shutting_down)
		{
			/* Bury zombie processes. */
			for (p = FIRST_PROC; p <= LAST_PROC; p++)
			{
				if ((p->state == PROC_ZOMBIE) && (p->father == curr_proc))
					bury(p);
			}
				
			/* Halt system. */
			if (nprocs == 1)
			{	
				kprintf("you may now turn off your computer");
				disable_interrupts();
				while (1)
					halt();
			}
		}
			
		halt();
		yield();
	}
}

/**
 * @brief Idle for SMP.
 */
PUBLIC void idle_smp(void)
{
	/**
	 * Core master should be able to receive interrupts
	 * while waiting for resources.
	 */
	pic_mask((1 << INT_COM1) | (1 << INT_OMPIC));
	mtspr(SPR_SR, mfspr(SPR_SR) | SPR_SR_TEE);
	enable_interrupts();

	while (1)
		halt();
}

/**
 * @brief Initializes the kernel.
 *
 * @param cmdline Command line parameters.
 */
PUBLIC void kmain(const char* cmdline)
{			
	if(!kstrcmp(cmdline,"debug"))
		dbg_init();

	/* Initialize system modules. */
	cpu_init();
	dev_init();
	mm_init();
	pm_init();
	fs_init();

	chkout(DEVID(TTY_MAJOR, 0, CHRDEV));

	dbg_execute();

	/* Spawn init process. */
	init();

	/* Init yield. */
	yield();
	
	/* Idle process accordingly to the architecture. */
	if (!smp_enabled)
		idle_up();
	else
		idle_smp();
}
