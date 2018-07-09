/*
 * Copyright(C) 2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#ifndef OMPIC_H_
#define OMPIC_H_

	#include <nanvix/mm.h>
	#include <nanvix/const.h>
	#include <stdint.h>

	/**
	 * OMPIC Registers and flags.
	 */
	#define OMPIC_CPUBYTES		8
	#define OMPIC_CTRL(cpu)		(OMPIC_VIRT + (0x0 + ((cpu) * OMPIC_CPUBYTES)))
	#define OMPIC_STAT(cpu)		(OMPIC_VIRT + (0x4 + ((cpu) * OMPIC_CPUBYTES)))

	#define OMPIC_CTRL_IRQ_ACK	(1 << 31)
	#define OMPIC_CTRL_IRQ_GEN	(1 << 30)
	#define OMPIC_CTRL_DST(cpu)	(((cpu) & 0x3fff) << 16)

	#define OMPIC_STAT_IRQ_PENDING (1 << 30)

	#define OMPIC_DATA(x)     ((x) & 0xffff)
	#define OMPIC_STAT_SRC(x) (((x) >> 16) & 0x3fff)

#ifndef _ASM_FILE_

	/* External functions. */
	EXTERN void ompic_init(void);
	EXTERN uint32_t ompic_readreg(uint32_t reg);
	EXTERN void ompic_writereg(uint32_t reg, uint32_t data);
	EXTERN void ompic_send_ipi(uint32_t dstcore, uint16_t data);
	EXTERN void ompic_handle_ipi(void);

#endif /* _ASM_FILE_ */

#endif /* OMPIC_H_ */
