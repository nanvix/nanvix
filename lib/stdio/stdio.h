/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * stdio/stdio.h - Private standard IO library.
 */

#ifndef _STDIO_H_
#define _STDIO_H_

	/*
	 * Asserts if a write buffer is full.
	 */
	#define WBUF_FULL(stream)        \
		((stream->nwritten < 0) ? 0 : \
		(stream->buffer[stream->nwritten] == stream->buffer[stream->bufsz]) ? 1 : 0) \

	/* File streams table. */
	extern FILE streams[FOPEN_MAX];

#endif /* STDIO_H_ */
