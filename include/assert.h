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
 * @brief Program diagnostics library.
 */

#ifndef ASSERT_H_
#define ASSERT_H_

	/**
	 * @defgroup assert assert.h
	 *
	 * @brief Program diagnostics library.
	 */
	/**@{*/

	/**
	 * @brief Asserts a condition.
	 *
	 * @param cond Condition to assert.
	 */
	#define assert(cond) ((cond) ?                                        \
			(void) 0 :                                                    \
			(void) _assertfail("assertion failed: %s, file %s, line %d\n",\
									#cond, __FILE__, __LINE__ ))

	/**@}*/

	/* Ignore assertions. */
	#ifdef NDEBUG
	#undef assert
		#define assert(ignore)((void) 0)
	#endif

	/* Forward definitions. */
	extern void _assertfail(const char *, const char *, const char *, int);

#endif /* ASSERT_H_ */
