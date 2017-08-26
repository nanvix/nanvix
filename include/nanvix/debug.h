/*
 * Copyright(C) 2017-2017 Clement Rouquier <clementrouquier@gmail.com>
 *              2017-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @file nanvix/debug.h
 * 
 * @brief Debug driver
 */

#ifndef NANVIX_DEBUG_H
#define NANVIX_DEBUG_H

	#include <nanvix/klib.h>

	/**
 	 * @brief Debug counters.
 	 */
	struct tst_count
	{
		unsigned int tst_pass;
		unsigned int tst_fail;
		unsigned int tst_skip;
	};

	/**
	 * @brief Opaque pointer to a debug function.
	 */
	typedef void (*debug_fn)(void);
	
	/* Forward definitions */
	EXTERN void dbg_init(void);
	EXTERN void dbg_register(debug_fn, char *);
	EXTERN void dbg_execute(void);
	EXTERN void tst_passed(void);
	EXTERN void tst_failed(void);
	EXTERN void tst_skipped(void);

#endif /* NANVIX_DEBUG_H */
