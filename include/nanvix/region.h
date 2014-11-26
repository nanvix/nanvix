/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * <nanvix/region.h> - Memory regions library.
 */

#ifndef REGION_H_
#define REGION_H_

#ifndef _ASM_FILE_

	#include <i386/i386.h>
	#include <nanvix/const.h>
	#include <sys/types.h>

	/* Memory region flags. */
	#define REGION_FREE      0x01 /* Region is free.         */
	#define REGION_SHARED    0x02 /* Region is shared.       */
	#define REGION_LOCKED    0x04 /* Region is locked.       */
	#define REGION_STICKY    0x08 /* Stick region.           */
	#define REGION_DOWNWARDS 0x10 /* Region grows downwards. */
	#define REGION_UPWARDS   0x20 /* Region grows upwards.   */
	
	/* Memory region dimensions. */
	#define REGION_PGTABS (4)                        /* # Page tables.   */
	#define REGION_SIZE   (REGION_PGTABS*PGTAB_SIZE) /* Size (in bytes). */
	
	/*
	 * Memory region.
	 */
	struct region
	{
		/* General information. */
		int flags;             /* Flags (see above).     */
		int count;             /* Reference count.       */
		size_t size;           /* Region size.           */
		struct pte *pgtab[4];  /* Underlying page table. */
		struct process *chain; /* Sleeping chain.        */
		
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
	
	/*
	 * Initializes memory regions.
	 */
	EXTERN void initreg(void);

	/*
	 * Locks a memory region.
	 */
	EXTERN void lockreg(struct region *reg);
	
	/*
	 * Unlocks a memory region.
	 */
	EXTERN void unlockreg(struct region *reg);
	
	/*
	 * Returns access permissions to a memory region.
	 */
	#define accessreg(p, r) \
		permission(r->mode, r->uid, r->gid, p, MAY_ALL, 0)
	
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

	/*
	 * Allocates a memory region.
	 */
	EXTERN struct region *allocreg(mode_t mode, size_t size, int flags);
	
	/*
	 * Frees a memory region.
	 */
	EXTERN void freereg(struct region *reg);

	/*
	 * Edits permissions on a memory region.
	 */
	PUBLIC int editreg(struct region *reg, uid_t uid, gid_t gid, mode_t mode);

	/*
	 * Attaches a memory region to a process.
	 */
	EXTERN int attachreg(struct process *proc, struct pregion *preg, addr_t addr, struct region *reg);
	
	/*
	 * Detaches a memory region from a process.
	 */
	EXTERN void detachreg(struct process *proc, struct pregion *preg);

	/*
	 * Duplicates a memory region.
	 */
	EXTERN struct region *dupreg(struct region *reg);

	/*
	 * Changes the size of a memory region.
	 */
	EXTERN int growreg(struct process *proc, struct pregion *preg, ssize_t size);
	
	/*
	 * Finds a memory region
	 */
	EXTERN struct pregion *findreg(struct process *proc, addr_t addr);
	
	/*
	 * Loads a portion of a file into a memory region.
	 */
	EXTERN int loadreg(struct inode *inode, struct region *reg, off_t off, size_t size);

#endif /* _ASM_FILE */

#endif /* REGION_H_ */
