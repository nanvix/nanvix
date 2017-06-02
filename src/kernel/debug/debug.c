/*
 * Copyright(C) 2017-2017 Clement Rouquier <clementrouquier@gmail.com>
 *              2011-2017 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
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

#include <nanvix/config.h>
#include <nanvix/klib.h>
#include <nanvix/debug.h>

/**
 * @brief Registered debugging functions.
 */
PRIVATE debug_fn debug_fns[DEBUG_MAX];

/**
 * @brief Is debug mode enabled?
 */
PRIVATE int is_debug = 0;

/**
 * @brief Registers a function in the debug driver.
 *
 * @param fn Function to register.
 */
PUBLIC void dbg_register(debug_fn fn)
{
	/* Nothing to do. */
	if (!is_debug)
	{
		kprintf("debug-driver: debug mode disabled");
		return;
	}

	/* Sanity check. */
	if (fn == NULL)
	{
		kprintf("debug-driver: register null debug function?");
		return;
	}

	/* Find an empty slot in the debug functions table. */
	for (int i = 0; i < DEBUG_MAX; i++)
	{
		/* Found. */
		if (debug_fns[i] == NULL)
		{
			debug_fns[i] = fn;
			return;
		}
	}

	kprintf("debug-driver: too many debug functions");
}

/**
 * @brief Initoalizes the debug driver
 */
PUBLIC void dbg_init(void)
{
	is_debug = 1;
	kprintf("debug-diver: debug driver intialized");
}

/**
 * @brief Executes registered debugging functions.
 */
PUBLIC void dbg_execute(void)
{
	int nexecuted = 0;

	/* Nothing to do. */
	if (!is_debug)
	{
		kprintf("debug-driver: debug mode disabled");
		return;
	}

	/* Execute registered debug functions. */
	for (int i = 0; i < DEBUG_MAX; i++)
	{
		/* Skip invalid entries. */
		if (debug_fns[i] == NULL)
			continue;

		kprintf("debug-driver: executing debug function %d", ++nexecuted);

		debug_fns[i]();
	}

	kprintf("debug-driver: done");
}

