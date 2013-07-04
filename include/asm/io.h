/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * io.h - Low-level I/O
 */

#ifndef IO_H_
#define IO_H_

	#include <i386/i386.h>
	#include <nanvix/const.h>

	/*
	 * DESCRIPTION:
	 *   The outputb() function writes a byte to a port.
	 * 
	 * RETURN VALUE:
	 *   The outputb() function returns no value.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN void outputb(word_t port, byte_t byte);
	
	/*
	 * DESCRIPTION:
	 *   The inputb() function reads a byte from a port.
	 * 
	 * RETURN VALUE:
	 *   The inputb() function returns the byte read.
	 * 
	 * ERRORS:
	 *   No errors are defined.
	 */
	EXTERN byte_t inputb(word_t port);

#endif /* IO_H_ */
