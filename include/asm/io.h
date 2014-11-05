/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * <asm/io.h> - Low-level I/O.
 */

#ifndef IO_H_
#define IO_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>

	/*
	 * Writes a byte to a port.
	 */
	EXTERN void outputb(word_t port, byte_t byte);
	
	/*
	 * Writes a word to a port.
	 */
	EXTERN void outputw(word_t port, word_t word);
	
	/*
	 * Reads a byte from a port.
	 */
	EXTERN byte_t inputb(word_t port);
	
	/*
	 * Reads a word from a port.
	 */
	EXTERN word_t inputw(word_t port);
	
	/*
	 * Forces the CPU to wait for an I/O operation to complete.
	 */
	EXTERN void iowait(void);

#endif /* IO_H_ */
