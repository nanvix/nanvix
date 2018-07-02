/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#include <nanvix/syscall.h>

/*
 * Join a specified thread.
 */
PUBLIC int pthread_join(pthread_t thread, void **retval)
{
	register unsigned r3
        __asm__("r3") = (unsigned) thread;

	register unsigned r4
        __asm__("r4") = (unsigned) retval;

	register int ret
		__asm__("r11") = NR_pthread_join;

	__asm__ volatile (
		"l.sys 1"
		: "=r" (ret)
        : "r" (ret),
          "r" (r3),
          "r" (r4)
	);

	return (ret);
}
