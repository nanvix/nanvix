/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

/**
 * @file
 *
 * @brief exit() implementation.
 */

#include <limits.h>
#include <stdlib.h>
#include <unistd.h>
#include "atexit.h"

/* Cleanup functions. */
extern void stdio_cleanup(void);  /* <stdio.h>  */
extern void dirent_cleanup(void); /* <dirent.h> */

/**
 * @brief Terminates the calling process.
 *
 * @param status Exit status.
 */
void exit(int status)
{
	/*
	 * Call registered atexit() functions
	 * in the reverse order in which they were
	 * registered.
	 */
	while (_atexit._ind-- > 0)
		_atexit._fns[_atexit._ind]();

	stdio_cleanup();

	_exit(status);
}
