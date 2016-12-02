/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * Copyright (c) 1990 Regents of the University of California.
 * All rights reserved.
 *
 * %sccs.include.redist.c%
 */

#include <stdlib.h>
#include "atexit.h"

/**
 * @brief Registers a function to run at process termination.
 *
 * @details Registers the function pointed to by @p fn, to be
 * called without arguments at normal program termination. 
 * At normal program termination, all functions registered by
 * the atexit() function are called, in the reverse order of
 * their registration, except that a function is called after
 * any previously registered functions that had already been
 * called at the time it was registered.
 *
 * @return Upon successful completion, atexit() returns 0;
 * otherwise, returns a non-zero value.
 */
int atexit(void (*fn)(void))
{
  return __register_exitproc (__et_atexit, fn, NULL, NULL);
}
