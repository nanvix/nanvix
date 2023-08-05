/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
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

#ifndef _HISTORY_H_
#define _HISTORY_H_

 	/**
 	 * @brief Command history.
 	 *
 	 * @note  This implementation is NOT a true stack per se. It acts as a
 	 *        stack till new additions reach STACK->top. After that, for each
 	 *        addition, the stack will remove an element from the bottom of
 	 *        it to accommodate the new element at the top (assuming the
 	 *        stack grows upwards).
 	 */
	struct history
	{
		int top;                 /**< Last logged command.  */
		int p;                   /**< Read pointer.         */
		int size;                /**< Maximum history size. */
		char *log[HISTORY_SIZE]; /**< History log.          */
	};

	/* Forward definitions. */
	extern struct history *history_init(int);
	extern void history_destroy(struct history *);
	extern void history_push(struct history *, char *);
	extern char *history_previous(struct history *);
	extern char *history_next(struct history *);

#endif /* _HISTORY_H_ */
