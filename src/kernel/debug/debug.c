/*
 * Copyright(C) 2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#include <nanvix/klib.h>


typedef void (*debug_fn)(void);

/**
 * @brief Array containing debugging functions
 */
PRIVATE debug_fn debug_fns[DEBUG_MAX];

/**
 * @brief Is debug mode enabled?
 */
PRIVATE int is_debug = 0;

/**
 * @brief Add a function to the debugging array
 * @param Function to add to the debugging array
 */
PUBLIC void dbg_register(debug_fn fn)
{
	if (!is_debug)
		return;

	int i;
	for(i=0;i<DEBUG_MAX;i++)
	{
		if(debug_fns[i] == NULL)
		{
			debug_fns[i] = fn;
			break;
		}
	}
	if(i == DEBUG_MAX)
		kprintf("too many debug functions");
}
/**
 * @brief Init debug process.
 */
PUBLIC void dbg_init(void)
{
	kprintf("debug: debug mode handled");
	is_debug = 1;
}

/**
 * @brief Print number of debugging functions then execute them
 */
PUBLIC void dbg_execute(void)
{
	if (is_debug)
	{
		int i,j;
		for(i=0;i<DEBUG_MAX;i++)
		{
			if(debug_fns[i] == NULL)
				break;

		}
		kprintf("debug: %d debug function(s) are going to be executed",i);
		for(j=0;j<i;j++)
			debug_fns[i]();
		kprintf("debug: %d debug functions had successfully been executed",j);
	}
	else
		kprintf("debug: debug mode disabled");
}

