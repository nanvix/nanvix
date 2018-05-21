/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2018-2018 Davidson Francis <davidsondfgl@gmail.com>
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

/* Exported symbols. */
.globl outputb
.globl outputw
.globl inputb
.globl inputw
.globl iowait

/*----------------------------------------------------------------------------*
 *                                  outputb                                   *
 *----------------------------------------------------------------------------*/

/*
 * Writes a byte to a port.
 */
outputb:
	l.jr r9
	l.nop

/*----------------------------------------------------------------------------*
 *                                  outputw                                   *
 *----------------------------------------------------------------------------*/

/*
 * Writes a word to a port.
 */
outputw:
	l.jr r9
	l.nop
	
/*----------------------------------------------------------------------------*
 *                                   inputb                                   *
 *----------------------------------------------------------------------------*/

/*
 * Reads a byte from a port.
 */
inputb:
	l.ori r11, r0, 0
	l.jr r9
	l.nop
	
/*----------------------------------------------------------------------------*
 *                                   inputw                                   *
 *----------------------------------------------------------------------------*/

/*
 * Reads a word from a port.
 */
inputw:
	l.ori r11, r0, 0
	l.jr r9
	l.nop
	
/*----------------------------------------------------------------------------*
 *                                   iowait                                   *
 *----------------------------------------------------------------------------*/

/*
 * Forces the CPU to wait for an I/O operation to complete.
 */
iowait:
	l.jr r9
	l.nop
