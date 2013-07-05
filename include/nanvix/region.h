/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * region.h - Memory regions header.
 */

#ifndef REGION_H_
#define REGION_H_

#ifndef _ASM_FILE_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <sys/types.h>

	/* Memory region flags. */
	#define REGION_FREE    0x01 /* Region is free.          */
	#define REGION_DONE    0x02 /* Region is initialized.   */
	#define REGION_SHARED  0x04 /* Region is shared.        */
	#define REGION_LOADING 0x08 /* Region is beeing loaded. */
	#define REGION_DEMAND  0x10 /* Region is on demand.     */
	#define REGION_LOCKED  0x20 /* Region is locked.        */
	
	/*
	 * DESCRIPTION:
	 *   The region structure describes a memory region.
	 */
	struct region
	{
		int flags;             /* Flags (see above).               */
		int count;             /* Reference count.                 */
		size_t size;           /* Region size (in pages).          */
		int nvalid;            /* Number of valid pages in region. */
		mode_t mode;           /* Read/write permission.           */
		uid_t cuid;            /* Creator's user ID.               */
		gid_t cgid;            /* Creator's group ID.              */
		pid_t cpid;            /* Creator's process ID.            */
		uid_t uid;             /* Owner's user ID.                 */
		gid_t gid;             /* Owner's group ID.                */
		struct pte *pgtab;     /* Underlying page table.           */
		struct process *chain; /* Sleeping chain.                  */
	};
	
	/* Process memory region types. */
	#define PREGION_UNUSED 0x0 /* Unused region.        */
	#define PREGION_TEXT   0x1 /* Text region.          */
	#define PREGION_DATA   0x2 /* Data region.          */
	#define PREGION_STACK  0x3 /* Stack region.         */
	#define PREGION_SHM    0x4 /* Shared memory region. */
	
	/*
	 * DESCRIPTION:
	 *   The pregion structure describes a process memory region.
	 */
	struct pregion
	{
		int type;           /* Type (see above).         */
		addr_t start;       /* Starting address.         */
		struct region *reg; /* Underlying memory region. */
	};

	EXTERN void initreg();

	EXTERN void lockreg(struct region *reg);
	
	EXTERN void unlockreg(struct region *reg);
	
	EXTERN int accessreg(struct process *proc, struct region *reg);

	EXTERN struct region *allocreg(int shared, mode_t mode, size_t size);

	EXTERN void freereg(struct region *reg);

	EXTERN int attachreg(struct process *proc, addr_t addr, int type, struct region *reg);
	
	EXTERN struct region *detachreg(struct process *proc, struct pregion *preg);

	EXTERN struct region *dupreg(struct region *reg);

	EXTERN int growreg(struct process *proc, struct pregion *preg, ssize_t size);
	
	EXTERN struct pregion *findreg(struct process *proc, addr_t addr);

#endif /* _ASM_FILE */

#endif /* REGION_H_ */
