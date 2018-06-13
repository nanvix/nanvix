/*
 * Copyright(C) 2011-2016 Pedro H. Penna    <pedrohenriquepenna@gmail.com>
 *              2015-2017 Davidson Francis  <davidsondfgl@hotmail.com>
 *              2017-2018 Guillaume Besnard <guillaume.besnard@protonmail.com>
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

#ifndef TMPTHREAD_H_
#define TMPTHREAD_H_

#ifndef _ASM_FILE_

	/**
     * @brief Thread user API
     *
     */
	extern int pthread_create(void *__pthread, void *__attr,
			 void *(*__start_routine)(void *), void *__arg);
#endif

#endif /* TMPTHREAD_H_ */
