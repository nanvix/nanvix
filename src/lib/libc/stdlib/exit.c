/*
 * Copyright(C) 2011-2015 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * @details Calls functions registered by atexit(), in the reverse order of
 *          their registration, except that a function is called after any
 *          previously registered functions that had already been called at
 *          the time it was registered. Each function is called as many times
 *          as it was registered. If, during the call to any such function, a
 *          call to the longjmp() function is made that would terminate the
 *          call to the registered function, the behavior is undefined.
 * 
 *          If a function registered by a call to atexit() fails to return,
 *          the remaining registered functions are not called and the rest of
 *          the exit() processing is completed. If exit() is called more than
 *          once, the behavior is undefined.
 * 
 *          The exit() function then flushes all open streams with unwritten
 *          buffered data and close all open streams. Finally, the process is 
 *          terminated
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
