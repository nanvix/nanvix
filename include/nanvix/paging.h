/*
 * Copyright(C) 2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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

#ifndef NANVIX_PAGING_H_
#define NANVIX_PAGING_H_

	#include <nanvix/const.h>
	#include <nanvix/pm.h>
	
	/* Forward definitions. */
	EXTERN int crtpgdir(struct process *);
	EXTERN int pfault(addr_t);
	EXTERN int vfault(addr_t);
	EXTERN void dstrypgdir(struct process *);
	PUBLIC void fillpg(struct pte *);
	EXTERN void freeupg(struct pte *);
	EXTERN void linkupg(struct pte *, struct pte *);
	EXTERN void putkpg(void *);
	EXTERN void mappgtab(struct process *, addr_t, void *);
	EXTERN void umappgtab(struct process *, addr_t);
	EXTERN void zeropg(struct pte *);
	EXTERN void *getkpg(int);

#endif /* NANVIX_PAGING_H_ */
