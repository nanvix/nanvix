/*
 * Copyright(C) 2011-2017 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>

/*
 * @brief Initializes the thread attributes.
 *
 * @details Initializes the thread attributes object pointed to by attr with
 * default attributes values.
 */
int pthread_attr_init(pthread_attr_t *attr)
{
	attr->detachstate = PTHREAD_CREATE_JOINABLE;
	attr->is_initialized = 1;
	return (0);
}

/*
 * @brief Cleanup the thread attributes structure.
 */
int pthread_attr_destroy(pthread_attr_t *attr)
{
	attr->is_initialized = 0;
	return (0);
}

/*
 * @brief Sets the detach state attribute.
 *
 * @details sets the detach state attribute of the thread attributes
 * object referred to by attr to the value specified in detachstate.
 *
 * @returns On success, these functions return 0; on error, they return a
 * nonzero error number.
 */
int pthread_attr_setdetachstate(pthread_attr_t *attr, int detachstate)
{
	if (detachstate != PTHREAD_CREATE_JOINABLE
			&& detachstate != PTHREAD_CREATE_DETACHED)
		return (EINVAL);
	attr->detachstate = detachstate;
	return (0);
}
