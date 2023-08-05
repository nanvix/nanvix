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

#ifndef GDT_H_
#define GDT_H_

	/* GDT entry size (in bytes). */
	#define GDTE_SIZE 8

	/* GDT pointer size (in bytes). */
	#define GDTPTR_SIZE 6

	/* GDT size (number of entries). */
	#define GDT_SIZE 6

	/* GDT entries. */
	#define GDT_NULL       0 /* Null.       */
	#define GDT_CODE_DPL0  1 /* Code DPL 0. */
	#define GDT_DATA_DPL0  2 /* Data DPL 0. */
	#define GDT_CODE_DPL3  3 /* Code DPL 3. */
	#define GDT_DATA_DPL3  4 /* Data DPL 3. */
	#define GDT_TSS        5 /* TSS.        */

	/* GDT segment selectors. */
	#define KERNEL_CS (GDTE_SIZE*GDT_CODE_DPL0)     /* Kernel code. */
	#define KERNEL_DS (GDTE_SIZE*GDT_DATA_DPL0)     /* Kernel data. */
	#define USER_CS   (GDTE_SIZE*GDT_CODE_DPL3 + 3) /* User code.   */
	#define USER_DS   (GDTE_SIZE*GDT_DATA_DPL3 + 3) /* User data.   */
	#define TSS       (GDTE_SIZE*GDT_TSS + 3)       /* TSS.         */

#ifndef _ASM_FILE_

	/*
	 * Global descriptor table entry.
	 */
	struct gdte
	{
		unsigned limit_low   : 16; /* Limit low.   */
		unsigned base_low    : 24; /* Base low.    */
		unsigned access      :  8; /* Access.      */
		unsigned limit_high  :  4; /* Limit high.  */
		unsigned granularity :  4; /* Granularity. */
		unsigned base_high   :  8; /* Base high.   */
	} __attribute__((packed));

	/*
	 * Global descritor table pointer.
	 */
	struct gdtptr
	{
		unsigned size : 16; /* GDT size.            */
		unsigned ptr  : 32; /* GDT virtual address. */
	} __attribute__((packed));

#endif /* _ASM_FILE_ */

#endif /* GDT_H_ */
