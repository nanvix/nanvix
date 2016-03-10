/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2015-2016 Davidson Francis <davidsondfgl@hotmail.com>
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

#ifndef FPU_H_
#define FPU_H_
#ifndef _ASM_FILE_

#define FPU_REGISTER 5

#include <nanvix/const.h>

/*
 * FPU Saved State.
 */
struct fpu
{
	unsigned cw  : 16; /* Control Word.        */
	unsigned un1 : 16; /* Unused.              */
	unsigned sw  : 16; /* Status Word.         */
	unsigned un2 : 16; /* Unused.              */
	unsigned tw  : 16; /* Tag Word.            */
	unsigned un3 : 16; /* Unused.              */
	unsigned fip : 32; /* Instruction Pointer. */
	unsigned fcs : 16; /* Code Segment.        */
	unsigned un4 : 16; /* Unused.              */
	unsigned foo : 32; /* Operand Address.     */
	unsigned fds : 16; /* Data Segment.        */
	unsigned un5 : 16; /* Unused.              */
	unsigned short st0[FPU_REGISTER]; /* ST(0) register. */
	unsigned short st1[FPU_REGISTER]; /* ST(1) register. */
	unsigned short st2[FPU_REGISTER]; /* ST(2) register. */
	unsigned short st3[FPU_REGISTER]; /* ST(3) register. */
	unsigned short st4[FPU_REGISTER]; /* ST(4) register. */
	unsigned short st5[FPU_REGISTER]; /* ST(5) register. */
	unsigned short st6[FPU_REGISTER]; /* ST(6) register. */
	unsigned short st7[FPU_REGISTER]; /* ST(7) register. */
} __attribute__((packed));

/*
 * Initializes a x87 Coprocessor
 */
EXTERN void fpu_init(void);

#endif /* _ASM_FILE_ */
#endif /* FPU_H_ */
