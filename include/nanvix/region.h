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

#ifndef REGION_H_
#define REGION_H_

#ifndef _ASM_FILE_

	#include <nanvix/const.h>
	#include <nanvix/hal.h>
	#include <sys/types.h>

	/* Memory region flags. */
	#define REGION_FREE      0x01 /* Region is free.         */
	#define REGION_SHARED    0x02 /* Region is shared.       */
	#define REGION_LOCKED    0x04 /* Region is locked.       */
	#define REGION_STICKY    0x08 /* Stick region.           */
	#define REGION_DOWNWARDS 0x10 /* Region grows downwards. */
	#define REGION_UPWARDS   0x20 /* Region grows upwards.   */

	/* Memory region dimensions. */
	#define REGION_PGTABS (8)                        /* # Page tables.   */
	#define REGION_SIZE   (REGION_PGTABS*PGTAB_SIZE) /* Size (in bytes). */

	/*
	 * Memory region.
	 */
	struct region
	{
		/* General information. */
		int flags;                        /* Flags (see above).          */
		int count;                        /* Reference count.            */
		size_t size;                      /* Region size.                */
		struct pte *pgtab[REGION_PGTABS]; /* Underlying page table.      */
		struct process *chain;            /* Sleeping chain.             */
		struct pregion *preg;             /* Process region attached to. */

		/* File information. */
		struct
		{
			struct inode *inode;   /* Inode.  */
			off_t off;             /* Offset. */
			size_t size;           /* Size.   */
		} file;

		/* Access information. */
		mode_t mode; /* Access permissions.      */
		uid_t cuid;  /* Creator's user ID.       */
		gid_t cgid;  /* Creator's group ID.      */
		uid_t uid;   /* Owner's user ID.         */
		gid_t gid;   /* Owner's group ID.        */
	};

	/*
	 * Process memory region.
	 */
	struct pregion
	{
		addr_t start;       /* Starting address.         */
		struct region *reg; /* Underlying memory region. */
	};

	/**
	 * @brief Returns access permissions to a memory region.
	 *
	 * @brief p Permissions to be queried.
	 * @brief r Memory region to be queried.
	 *
	 * @returns True if access is allowed and false otherwise.
	 */
	#define accessreg(p, r) \
		(permission(r->mode, r->uid, r->gid, p, MAY_ALL, 0))

	/**
	 * @brief Asserts if an address is withing a memory region.
	 *
	 * @param preg Process region to be queried.
	 * @param addr Address to be checked.
	 *
	 * @returns True if the address resides inside the memory region, and false
	 *          otherwise.
	 */
	#define withinreg(preg, addr)                            \
		(((preg)->reg->flags & REGION_DOWNWARDS) ?           \
			(((addr) <= (preg)->start) &&                    \
			((addr) >= (preg)->start - (preg)->reg->size)) : \
			(((addr) >= (preg)->start) &&                    \
			((addr) < (preg)->start + (preg)->reg->size)))   \

	/* Forward definitions. */
	EXTERN int attachreg(struct process*,struct pregion*,addr_t,struct region*);
	EXTERN int editreg(struct region *, uid_t, gid_t, mode_t);
	EXTERN int growreg(struct process *, struct pregion *, ssize_t);
	EXTERN int loadreg(struct inode *, struct region *, off_t, size_t);
	EXTERN void detachreg(struct process *, struct pregion *);
	EXTERN void freereg(struct region *);
	EXTERN void initreg(void);
	EXTERN void lockreg(struct region *);
	EXTERN void unlockreg(struct region *);
	EXTERN struct region *allocreg(mode_t, size_t, int);
	EXTERN struct region *dupreg(struct region *);
	EXTERN struct pregion *findreg(struct process *, addr_t);

#endif /* _ASM_FILE */

#endif /* REGION_H_ */
