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

	#include <nanvix/const.h>

	#include <stdint.h>

	/**
	 * @brief FPU register width.
	 */
	#define FPU_REGISTER_WIDTH 5

	/**
	 * @brief FPU state.
	 */
	struct fpu
	{
		uint16_t cw;                      /** Control Word.        */
		uint16_t un1;                     /** Unused.              */
		uint16_t sw;                      /** Status Word.         */
		uint16_t un2;                     /** Unused.              */
		uint16_t tw;                      /** Tag Word.            */
		uint16_t un3;                     /** Unused.              */
		uint32_t fip;                     /** Instruction Pointer. */
		uint16_t fcs;                     /** Code Segment.        */
		uint16_t un4;                     /** Unused.              */
		uint32_t foo;                     /** Operand Address.     */
		uint16_t fds;                     /** Data Segment.        */
		uint16_t un5;                     /** Unused.              */
		uint16_t st0[FPU_REGISTER_WIDTH]; /** ST(0) register.      */
		uint16_t st1[FPU_REGISTER_WIDTH]; /** ST(1) register.      */
		uint16_t st2[FPU_REGISTER_WIDTH]; /** ST(2) register.      */
		uint16_t st3[FPU_REGISTER_WIDTH]; /** ST(3) register.      */
		uint16_t st4[FPU_REGISTER_WIDTH]; /** ST(4) register.      */
		uint16_t st5[FPU_REGISTER_WIDTH]; /** ST(5) register.      */
		uint16_t st6[FPU_REGISTER_WIDTH]; /** ST(6) register.      */
		uint16_t st7[FPU_REGISTER_WIDTH]; /** ST(7) register.      */
	} __attribute__((packed));

	EXTERN void fpu_init(void);

#endif /* _ASM_FILE_ */
#endif /* FPU_H_ */
