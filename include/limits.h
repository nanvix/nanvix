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

#ifndef LIMITS_H_
#define LIMITS_H_

	/* Number of functions that may be registered with atexit() */
	#define ATEXIT_MAX 32

	/* Default process priority. */
	#define NZERO 20

	/* Maximum number of links to a single file. */
	#define LINK_MAX 8

	/* Maximum value of a long. */
	#define LONG_MAX 2147483647

	/* Minimum value of type long. */
	#define LONG_MIN -2147483647

	/* Maximum value for an object of type int. */
	#define INT_MAX 2147483647

	/* Minimum value for an object of type int. */
	#define INT_MIN -2147483647

	/**
	 * @brief Maximum value for an object of type long long.
	 */
	#define LLONG_MAX +9223372036854775807

	/**
	 * @brief Minimum value for an object of type long long.
	 */
	#define LLONG_MIN -9223372036854775807

	/**
	 * @brief Maximum number of bytes in a filename.
	 */
	#define NAME_MAX 14

	/* Files that one process can have open simultaneously. */
	#define OPEN_MAX 20

	/* Length of argument to the execve(). */
	#define ARG_MAX 2048

	/**
	 * @brief Maximum number of bytes in a pathname.
	 */
	#define PATH_MAX 512

	/* Maximum value for unsigned long. */
	#define ULONG_MAX 4294967295u

	/**
	 * @brief Maximum value for an object of type unsigned long long.
	 */
	#define ULLONG_MAX 18446744073709551615u

#endif /* LIMITS_H_ */
