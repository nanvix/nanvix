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
#include <stdio.h>
#include <pthread.h>

/*
 * @brief Init routine called before executing the thread supplied routine.
 *
 * @details This can be seen as thread crt0, but set at runtime instead.
 * It is useful here to be able to implicitly called pthread_exit, when
 * start_routine return.
 *
 * @returns This routine will never return.
 */
void __start_thread(void *(*__start_routine)(void *), void *arg)
{
	void *ret;
	ret = __start_routine(arg);
	pthread_exit(ret);
}

/*
 * Creates a new thread.
 */
int pthread_create(pthread_t *__pthread, _CONST pthread_attr_t *__attr,
				   void *(*__start_routine)(void *), void *__arg)
{
	((void)__start_routine);
	register unsigned r3
        __asm__("r3") = (unsigned) __pthread;
    register unsigned r4
        __asm__("r4") = (unsigned) __attr;
    register unsigned r5
        __asm__("r5") = (unsigned) __start_routine;
	register unsigned r6
        __asm__("r6") = (unsigned) __arg;
	register unsigned r7
        __asm__("r7") = (unsigned) __start_thread;


	register int ret
		__asm__("r11") = NR_pthread_create;

	__asm__ volatile (
		"l.sys 1"
		: "=r" (ret)
        : "r" (ret),
          "r" (r3),
          "r" (r4),
          "r" (r5),
          "r" (r6),
          "r" (r7)
	);

	return (ret);
}
