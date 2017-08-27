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
#include <nanvix/debug.h>
#include <nanvix/dev.h>
#include <nanvix/klib.h>


/**
 * @brief Registered debugging functions.
 */
PRIVATE debug_fn debug_fns[DEBUG_MAX];

/**
 * @brief Name of registed debugging functions.
 */
PRIVATE char *fn_names[DEBUG_MAX];

/**
 * @brief Failed debugging functions.
 */
PRIVATE int failed_fns[DEBUG_MAX];

/**
 * Current debugging function.
 */
PRIVATE int current_fn;

/**
 * @brief Debugging driver counters.
 */
PRIVATE struct tst_count tst_cnt;

/**
 * @brief Is debug mode enabled?
 */
PRIVATE int is_debug = 0;


/**
 * @brief Increments the counter for passed tests.
 */
PUBLIC void tst_passed(void) 
{ 
	tst_cnt.tst_pass++; 
}

/**
 * @brief Increments the counter for failed tests.
 */
PUBLIC void tst_failed(void) 
{ 
	tst_cnt.tst_fail++;
	failed_fns[(tst_cnt.tst_fail)-1] = current_fn;
}

/**
 * @brief Increments the counter for skipped tests.
 */
PUBLIC void tst_skipped(void) 
{
	tst_cnt.tst_skip++; 
}

/**
 * @brief Registers a function in the debug driver.
 *
 * @param fn Function to register.
 */
PUBLIC void dbg_register(debug_fn fn, char *fn_name)
{
	/* Sanity check. */
	if (fn == NULL)
	{
		kprintf(KERN_WARNING "debug-driver: register null debug function?");
		return;
	}

	/* Nothing to do. */
	if (!is_debug)
	{
		kprintf(KERN_INFO "debug-driver: debug mode disabled");
		return;
	}

	/* Find an empty slot in the debug functions table. */
	for (int i = 0; i < DEBUG_MAX; i++)
	{
		/* Found. */
		if (debug_fns[i] == NULL)
		{
			fn_names[i] = fn_name;
			debug_fns[i] = fn;
			return;
		}
	}

	kprintf(KERN_ERR "debug-driver: too many debug functions");
}

/**
 * @brief Initializes the debug driver
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
	int i;
	int nexecuted = 0;

	/* Nothing to do. */
	if (!is_debug)
	{
		kprintf(KERN_ERR "debug-driver: debug mode disabled");
		return;
	}

	/* Execute registered debug functions. */
	for (int i = 0; i < DEBUG_MAX; i++)
	{
		/* Skip invalid entries. */
		if (debug_fns[i] == NULL)
			continue;

		current_fn = i;

		kprintf(KERN_DEBUG "debug-driver: executing debug function %d: %s", ++nexecuted, fn_names[i]);
		debug_fns[i]();
	}

	kprintf(KERN_DEBUG "debug-driver: done - test passed: %d - test failed: %d - test skipped: %d", tst_cnt.tst_pass, tst_cnt.tst_fail, tst_cnt.tst_skip);

	if (tst_cnt.tst_fail > 0)
	{
		kprintf(KERN_DEBUG "debug-driver: list of failed functions:");
		for(i = 0; i < (int)(tst_cnt.tst_fail); i++)
			kprintf(KERN_DEBUG "debug-driver: failed function %d:  %s", (i+1),fn_names[failed_fns[i]]);
		kpanic(KERN_DEBUG "debug-driver: failed tests could create instability in the system");
	}

	kprintf(KERN_DEBUG "debug-driver: debug complete - press ENTER to continue standard boot");

	cdev_read(kout, NULL, 1);
}

