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
PUBLIC volatile spinlock_t boot_lock;

/*
 * @brief Gets the core number of the current processor.
 * @return Core number of the current processor.
 */
PUBLIC unsigned smp_get_coreid(void)
{
	return 0;
}

/*
 * @brief Gets the number of cores available in the system.
 * @return Number of cores.
 */
PUBLIC unsigned smp_get_numcores(void)
{
	return 1;
}

/*
 * @brief Initializes the SMP system if available.
 */
PUBLIC void smp_init(void)
{
}
