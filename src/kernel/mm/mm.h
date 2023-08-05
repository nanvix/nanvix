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

#ifndef _MM_H_
#define _MM_H_

	/* Page marks. */
	#define PAGE_FILL 0 /* Demand fill. */
	#define PAGE_ZERO 1 /* Demand zero. */

	/* Forward definitions. */
	EXTERN void freeupg(struct pte *);
	EXTERN void linkupg(struct pte *, struct pte *);
	EXTERN void mappgtab(struct process *, addr_t, void *);
	EXTERN void markpg(struct pte *, int);
	EXTERN void umappgtab(struct process *, addr_t);

#endif /* _MM_H_ */
