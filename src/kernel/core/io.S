/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * io.S - Low-level I/O
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
