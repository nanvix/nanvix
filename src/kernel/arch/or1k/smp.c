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

#include <or1k/or1k.h>
#include <or1k/ompic.h>
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/smp.h>

/**
 * @brief SMP enabled.
 */
PUBLIC unsigned smp_enabled = 0;

/**
 * @brief Release CPU accordingly to the current value.
 */
PUBLIC unsigned release_cpu = -1;

/**
 * @brief Boot-lock, spin-lock that synchronizes the CPUs
 * initialization.
 */
PUBLIC spinlock_t boot_lock;

/*
 * @brief Gets the core number of the current processor.
 * @return Core number of the current processor.
 */
PUBLIC unsigned smp_get_coreid(void)
{
	register unsigned coreid
		__asm__("r11") = SPR_COREID;
	
	__asm__ volatile (
		"l.mfspr r11, r11, 0"
		: "=r" (coreid)
		: "r" (coreid)
	);

	return (coreid);
}

/*
 * @brief Gets the number of cores available in the system.
 * @return Number of cores.
 */
PUBLIC unsigned smp_get_numcores(void)
{
	register unsigned numcores
		__asm__("r11") = SPR_NUMCORES;
	
	__asm__ volatile (
		"l.mfspr r11, r11, 0"
		: "=r" (numcores)
		: "r" (numcores)
	);

	return (numcores);
}

/*
 * @brief Initializes the SMP system if available.
 */
PUBLIC void smp_init(void)
{
	unsigned numcores = smp_get_numcores();

	kprintf("%d CPUs detected!", numcores);

	/* Check if we have more than one core and if so, enables SMP. */
	if (numcores > 1)
	{
		/* Ensures that we have at least one shadow gpr file. */
		if ( !(mfspr(SPR_CPUCFGR) & SPR_CPUCFGR_NSGF) )
			kpanic("smp: shadow registers are not supported!");

		spin_init(&boot_lock);
		ompic_init();

		kprintf("  -- enabling core #0");
		
		for (unsigned cpu = 1; cpu < numcores; cpu++)
		{
			spin_lock(&boot_lock);
			release_cpu = cpu;	
			kprintf("  -- enabling core #%d", cpu);
		
			/* the cpu released will unlock the spin-lock. */
		}

		spin_lock(&boot_lock);
		spin_unlock(&boot_lock);

		smp_enabled = 1;
	}
}
