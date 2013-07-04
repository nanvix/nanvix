/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * io.S - Low-level I/O
 */

/* Exported symbols. */
.globl outputb
.globl inputb

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
