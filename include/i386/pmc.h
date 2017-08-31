/*
 * Copyright(C) 2016 Davidson Francis <davidsondfgl@gmail.com>
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

/**
 * @file i386/pmc.h
 * 
 * @brief Performance Monitor Counter.
 */

#ifndef PMC_H_
#define PMC_H_

	/**
	 * @name Performance MSRs.
	 */
	/**@{*/
	#define IA32_PERFEVTSELx          0x00000186 /**< Performance event select.       */
	#define IA32_PMCx                 0x000000c1 /**< Performance monitoring counter. */
	#define IA32_PERF_GLOBAL_CTRL     0x0000038f /**< Performance global control.     */
	#define IA32_PERF_GLOBAL_OVF_CTRL 0x00000390 /**< Performance overflow control.   */
	/**@}*/

	/**
	 * @name Performance event select flags.
	 */
	/**@{*/
	#define IA32_PERFEVTSELx_EN   0x400000  /**< Enable counter.           */
	#define IA32_PERFEVTSELx_OS   0x020000  /**< Count in ring 0.          */
	#define IA32_PERFEVTSELx_USR  0x010000  /**< Count in ring 1, 2 and 3. */
	/**@}*/

	/**
	 * @name Performance counters.
	 */
	/**@{*/
	#define IA32_PMC0 0x1 /**< Enable PMC0. */
	#define IA32_PMC1 0x2 /**< Enable PMC1. */
	/**@}*/

#ifndef _ASM_FILE_

	#include <nanvix/const.h>
	#include <stdint.h>

	/**
	 * @name Architectural Performance-Monitoring Events.
	 *
	 * @note This type of event was introduced in Intel Core Solo 
	 * and Core Duo and are also supported on processors based on
	 * the Intel Core microarchitecture.
	 *
	 * The following values have 16 bits, the most significant byte
	 * is relative to the 'Unit Mask' field and the least significant
	 * byte is relative to the 'Event Select' field.
	 *
	 * Please refer to Intel Architectures SW Developer's Manual, Volume 3,
	 * Chapters 18-19 for more information.
	 */
	/**@{*/
	#define UNHALTED_CORE_CYCLES       0x003C
	#define INSTRUCTION_RETIRED        0x00C0
	#define UNHALTED_REFERENCE_CYCLES  0x013C
	#define LLC_REFERENCE              0x4F2E
	#define LLC_MISSES                 0x412E
	#define BRANCH_INSTRUCTION_RETIRED 0x00C4
	#define BRANCH_MISSES_RETIRED      0x00C5
	/**@}*/

	/**
	 * @brief PMC structure.
	 */
	struct pmc
	{
		uint8_t  enable_counters; /**< Bit 0 - C1 / Bit 1 - C2 */
		uint16_t event_C1;        /**< Counter 0 event.        */
		uint16_t event_C2;        /**< Counter 1 event.        */
		uint64_t C1;              /**< Counter 0 value.        */
		uint64_t C2;              /**< Counter 1 value.        */
	} __attribute__((packed));

	/**
	 * @name acct() parameters.
	 */
	/**@{*/
	#define ACCT_WR 0 /**< Configure PMCs. */
	#define ACCT_RD 1 /**< Read counters.  */
	/**@}*/

#ifdef BUILDING_KERNEL
	EXTERN void write_msr(uint32_t, uint64_t);
	EXTERN uint64_t read_msr(uint32_t);
	EXTERN uint64_t read_pmc(int);
	EXTERN void pmc_init(void);
#else
	EXTERN int acct(struct pmc *, unsigned char);
#endif
	
#endif /* _ASM_FILE_ */
#endif /* PMC_H_ */
