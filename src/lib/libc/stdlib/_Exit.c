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
 * @brief _Exit() implementation.
 */

#include <unistd.h>

/**
 * @brief Terminates the calling process.
 * 
 * @details Terminates the calling process. Functions registered with atexit()
 *          nor any registered signal handlers are not called. Open streams are
 *          flushed. Open streams are not closed. Finally, the calling process
 *          is terminated.
 * 
 *          The value of status may be 0, EXIT_SUCCESS, EXIT_FAILURE, or any
 *          other value, though only the least significant 8 bits are available
 *          to a waiting parent process.
 * 
 * @note The _Exit() and _exit() functions are functionally equivalent.
 */
void _Exit(int status)
{
	_exit(status);
}
