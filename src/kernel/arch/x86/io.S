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
	pushl %edx
	movl  8(%esp), %edx /* Port number. */
	movl 12(%esp), %eax /* Byte.        */
	outb %al, %dx
	popl %edx
	ret

/*----------------------------------------------------------------------------*
 *                                  outputw                                   *
 *----------------------------------------------------------------------------*/

/*
 * Writes a word to a port.
 */
outputw:
	pushl %edx
	movl  8(%esp), %edx /* Port number. */
	movl 12(%esp), %eax /* Word.        */
	outw %ax, %dx
	popl %edx
	ret

/*----------------------------------------------------------------------------*
 *                                   inputb                                   *
 *----------------------------------------------------------------------------*/

/*
 * Reads a byte from a port.
 */
inputb:
	pushl %edx
	movl  8(%esp), %edx /* Port number. */
	inb  %dx, %al
	popl %edx
	ret

/*----------------------------------------------------------------------------*
 *                                   inputw                                   *
 *----------------------------------------------------------------------------*/

/*
 * Reads a word from a port.
 */
inputw:
	pushl %edx
	movl  8(%esp), %edx /* Port number. */
	inw  %dx, %ax
	popl %edx
	ret

/*----------------------------------------------------------------------------*
 *                                   iowait                                   *
 *----------------------------------------------------------------------------*/

/*
 * Forces the CPU to wait for an I/O operation to complete.
 */
iowait:
	xorl %eax, %eax
	outb %al, $0x80
	ret
