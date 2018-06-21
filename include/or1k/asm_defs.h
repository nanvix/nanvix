/*
 * Copyright(C) 2017-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef ASM_DEFS_H
#define ASM_DEFS_H

/* Shadow register. */
#define SPR_SHADOW_GPR(x) ((x) + SPR_GPR_BASE + 32)

/*
 * Symbols
 */
#define CLEAR_GPR(gpr)	\
	l.or    gpr,r0,r0

#define LOAD_SYMBOL_2_GPR(gpr,symbol) \
	l.movhi gpr,hi(symbol)			 ;\
	l.ori   gpr,gpr,lo(symbol)

#define CLEAR_ALL_GPR \
	l.or r6,  r0, r0 ;\
	l.or r7,  r0, r0 ;\
	l.or r8,  r0, r0 ;\
	l.or r9,  r0, r0 ;\
	l.or r10, r0, r0 ;\
	l.or r11, r0, r0 ;\
	l.or r12, r0, r0 ;\
	l.or r13, r0, r0 ;\
	l.or r14, r0, r0 ;\
	l.or r15, r0, r0 ;\
	l.or r16, r0, r0 ;\
	l.or r17, r0, r0 ;\
	l.or r18, r0, r0 ;\
	l.or r19, r0, r0 ;\
	l.or r20, r0, r0 ;\
	l.or r21, r0, r0 ;\
	l.or r22, r0, r0 ;\
	l.or r23, r0, r0 ;\
	l.or r24, r0, r0 ;\
	l.or r25, r0, r0 ;\
	l.or r26, r0, r0 ;\
	l.or r27, r0, r0 ;\
	l.or r28, r0, r0 ;\
	l.or r29, r0, r0 ;\
	l.or r30, r0, r0 ;\
	l.or r31, r0, r0
 
/*
 * Enable MMU and cache when exception occurs.
 */
#define ENABLE_MMU (SPR_SR_DME | SPR_SR_IME | SPR_SR_DCE | SPR_SR_ICE)

/*
 * User SR: Initial SR register for user space program.
 */
#define USER_SR (ENABLE_MMU | SPR_SR_IEE | SPR_SR_TEE)

/*
 * TLB miss handlers temorary stores
 */
#ifdef OR1K_HAVE_SHADOW_GPRS
	#define EXCEPTION_STORE_GPR9  l.mtspr r0, r9, SPR_SHADOW_GPR(9)
	#define EXCEPTION_LOAD_GPR9	  l.mfspr r9, r0, SPR_SHADOW_GPR(9)

	#define EXCEPTION_STORE_GPR2  l.mtspr r0, r2, SPR_SHADOW_GPR(2)
	#define EXCEPTION_LOAD_GPR2	  l.mfspr r2, r0, SPR_SHADOW_GPR(2)

	#define EXCEPTION_STORE_GPR3  l.mtspr r0, r3, SPR_SHADOW_GPR(3)
	#define EXCEPTION_LOAD_GPR3	  l.mfspr r3, r0, SPR_SHADOW_GPR(3)

	#define EXCEPTION_STORE_GPR4  l.mtspr r0, r4, SPR_SHADOW_GPR(4)
	#define EXCEPTION_LOAD_GPR4   l.mfspr r4, r0, SPR_SHADOW_GPR(4)

	#define EXCEPTION_STORE_GPR5  l.mtspr r0, r5, SPR_SHADOW_GPR(5)
	#define EXCEPTION_LOAD_GPR5   l.mfspr r5, r0, SPR_SHADOW_GPR(5)

	#define EXCEPTION_STORE_GPR6  l.mtspr r0, r6, SPR_SHADOW_GPR(6)
	#define EXCEPTION_LOAD_GPR6   l.mfspr r6, r0, SPR_SHADOW_GPR(6)
#else
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
#endif


/*
 * EXCEPTION_HANDLE temporary stores
 */
#ifdef OR1K_HAVE_SHADOW_GPRS
	#define EXCEPTION_T_STORE_GPR30     l.mtspr r0, r30, SPR_SHADOW_GPR(30)
	#define EXCEPTION_T_LOAD_GPR30(reg)	l.mfspr reg, r0, SPR_SHADOW_GPR(30)

	#define EXCEPTION_T_STORE_GPR10     l.mtspr r0, r10, SPR_SHADOW_GPR(10)
	#define EXCEPTION_T_LOAD_GPR10(reg)	l.mfspr reg, r0, SPR_SHADOW_GPR(10)

	#define EXCEPTION_T_STORE_SP        l.mtspr r0, r1, SPR_SHADOW_GPR(1)
	#define EXCEPTION_T_LOAD_SP(reg)    l.mfspr reg, r0, SPR_SHADOW_GPR(1)
#else
	#define EXCEPTION_T_STORE_GPR30     l.sw    0x78(r0),r30
	#define EXCEPTION_T_LOAD_GPR30(reg)	l.lwz   reg,0x78(r0)

	#define EXCEPTION_T_STORE_GPR10     l.sw    0x7c(r0),r10
	#define EXCEPTION_T_LOAD_GPR10(reg)	l.lwz   reg,0x7c(r0)

	#define EXCEPTION_T_STORE_SP        l.sw    0x80(r0),r1
	#define EXCEPTION_T_LOAD_SP(reg)    l.lwz   reg,0x80(r0)
#endif

#endif	/* ASM_DEFS_H */
