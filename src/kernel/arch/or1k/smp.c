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
#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>

/**
 * @brief SMP enabled.
 */
PUBLIC unsigned smp_enabled = 0;

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
	/* Check if we have more than one core and if so, enables SMP. */
	if (smp_get_numcores() > 1)
	{
		smp_enabled = 1;
		
		kprintf("SMP enabled... core #%d", smp_get_coreid());
		
		for (unsigned i = 1; i < smp_get_numcores(); i++)
			kprintf("  -- enabling core #%d", i);
	}
	else
		kprintf("SMP not enabled");
}
