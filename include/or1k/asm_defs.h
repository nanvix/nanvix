/*
 * Copyright(C) 2017-2017 Davidson Francis <davidsondfgl@gmail.com>
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

/*
 * OpenRISC head.S
 *
 * Linux architectural port borrowing liberally from similar works of
 * others.  All original copyrights apply as per the original source
 * declaration.
 *
 * Modifications for the OpenRISC architecture:
 * Copyright (C) 2003 Matjaz Breskvar <phoenix@bsemi.com>
 * Copyright (C) 2010-2011 Jonas Bonn <jonas@southpole.se>
 *
 *      This program is free software; you can redistribute it and/or
 *      modify it under the terms of the GNU General Public License
 *      as published by the Free Software Foundation; either version
 *      2 of the License, or (at your option) any later version.
 */

/*
 * Symbols
 */
#define CLEAR_GPR(gpr)	\
	l.or    gpr,r0,r0

#define LOAD_SYMBOL_2_GPR(gpr,symbol) \
	l.movhi gpr,hi(symbol)			 ;\
	l.ori   gpr,gpr,lo(symbol)

/*
 * Enable MMU and cache when exception occurs.
 */
#define ENABLE_MMU (SPR_SR_DME | SPR_SR_IME | SPR_SR_DCE | SPR_SR_ICE)

/*
 * User SR: Initial SR register for user space program.
 */
#define USER_SR (ENABLE_MMU | SPR_SR_IEE | SPR_SR_TEE)

/*
 * emergency_print temporary stores
 */
#define EMERGENCY_PRINT_STORE_GPR4	l.sw    0x20(r0),r4
#define EMERGENCY_PRINT_LOAD_GPR4	l.lwz   r4,0x20(r0)

#define EMERGENCY_PRINT_STORE_GPR5	l.sw    0x24(r0),r5
#define EMERGENCY_PRINT_LOAD_GPR5	l.lwz   r5,0x24(r0)

#define EMERGENCY_PRINT_STORE_GPR6	l.sw    0x28(r0),r6
#define EMERGENCY_PRINT_LOAD_GPR6	l.lwz   r6,0x28(r0)

#define EMERGENCY_PRINT_STORE_GPR7	l.sw    0x2c(r0),r7
#define EMERGENCY_PRINT_LOAD_GPR7	l.lwz   r7,0x2c(r0)

#define EMERGENCY_PRINT_STORE_GPR8	l.sw    0x30(r0),r8
#define EMERGENCY_PRINT_LOAD_GPR8	l.lwz   r8,0x30(r0)

#define EMERGENCY_PRINT_STORE_GPR9	l.sw    0x34(r0),r9
#define EMERGENCY_PRINT_LOAD_GPR9	l.lwz   r9,0x34(r0)


/*
 * TLB miss handlers temorary stores
 */
#define EXCEPTION_STORE_GPR9  l.sw    0x10(r0),r9
#define EXCEPTION_LOAD_GPR9	  l.lwz   r9,0x10(r0)

#define EXCEPTION_STORE_GPR2  l.sw    0x64(r0),r2
#define EXCEPTION_LOAD_GPR2	  l.lwz   r2,0x64(r0)

#define EXCEPTION_STORE_GPR3  l.sw    0x68(r0),r3
#define EXCEPTION_LOAD_GPR3	  l.lwz   r3,0x68(r0)

#define EXCEPTION_STORE_GPR4  l.sw    0x6c(r0),r4
#define EXCEPTION_LOAD_GPR4   l.lwz   r4,0x6c(r0)

#define EXCEPTION_STORE_GPR5  l.sw    0x70(r0),r5
#define EXCEPTION_LOAD_GPR5   l.lwz   r5,0x70(r0)

#define EXCEPTION_STORE_GPR6  l.sw    0x74(r0),r6
#define EXCEPTION_LOAD_GPR6   l.lwz   r6,0x74(r0)


/*
 * EXCEPTION_HANDLE temporary stores
 */

#define EXCEPTION_T_STORE_GPR30     l.sw    0x78(r0),r30
#define EXCEPTION_T_LOAD_GPR30(reg)	l.lwz   reg,0x78(r0)

#define EXCEPTION_T_STORE_GPR10     l.sw    0x7c(r0),r10
#define EXCEPTION_T_LOAD_GPR10(reg)	l.lwz   reg,0x7c(r0)

#define EXCEPTION_T_STORE_SP        l.sw    0x80(r0),r1
#define EXCEPTION_T_LOAD_SP(reg)    l.lwz   reg,0x80(r0)

/*
 * For UNHANLDED_EXCEPTION
 */

#define EXCEPTION_T_STORE_GPR31		l.sw     0x84(r0),r31
#define EXCEPTION_T_LOAD_GPR31(reg)	l.lwz    reg,0x84(r0)
