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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

/**
 * @file nanvix/debug.h
 * 
 * @brief Debug mode
 */

#ifndef NANVIX_DEBUG_H
#define NANVIX_DEBUG_H

	#include <nanvix/klib.h>

	typedef void (*debug_fn)(void);
	
	/* Forward definitions */
	EXTERN void dbg_init(void);
	EXTERN void dbg_register(debug_fn fn);
	EXTERN void dbg_execute();

#endif /* NANVIX_DEBUG_H */
