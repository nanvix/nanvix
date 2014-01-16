/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stdio.h> - Standard IO library.
 */

#ifndef STDIO_H_
#define STDIO_H_

	#include <sys/types.h>
	#include <limits.h>

	/* Standard buffer size. */
	#define BUFSIZ 256

	/* End of file. */
	#define EOF -1
	
	/* Maximum number of opened file streams. */
	#define FOPEN_MAX OPEN_MAX

	/* File stream flags. */
	#define _IOFBF     00001 /* Fully buffered?                */
	#define _IOLBF     00002 /* Line buffered?                 */
	#define _IONBF     00004 /* Unbuffered?                    */
	#define _IOREAD    00010 /* Readable?                      */
	#define _IOWRITE   00020 /* Writable?                      */
	#define _IOAPPEND  00040 /* Append?                        */
	#define _IOEOF     00100 /* End of file reached?           */
	#define _IOERROR   00200 /* Error encountered?             */
	#define _IO_MYBUF  00400 /* Library buffer?                */
	#define _IOREADING 01000 /* Now reading?                   */
	#define _IOWRITING 02000 /* Now writing?                   */
	#define _IOSYNC    04000 /* Sync file position on append?  */

	/*
	 * File stream.
	 */
	typedef struct _iobuf
	{
		int fd;        /* File descriptor.    */
		int flags;     /* Flags (see above).  */
		char *buf;     /* Stream buffer.      */
		char *ptr;     /* Next character.     */
		size_t bufsiz; /* Buffer size.        */
		int nread;     /* Read characters.    */
		int nwritten;  /* Written characters. */
	} FILE;
	
	/*
	 * Writes a character to a file.
	 */
	extern int fputc(int c, FILE *stream);
	
	/*
	 * Writes a string to a file.
	 */
	extern int fputs(const char *str, FILE *stream);
	
	/*
	 * Writes a character to a file.
	 */
	extern int putc(int c, FILE *stream);
	
	/*
	 * Writes a character to the standard output file.
	 */
	extern int putchar(int c);
	
	/*
	 * Writes a string to the standard output file.
	 */
	extern int puts(const char *str);
	
	/*
	 * Flushes a file stream.
	 */
	extern int fflush(FILE *stream);
	
	/* Standard file streams. */
	extern FILE *stdin;  /* Standard input.  */
	extern FILE *stdout; /* Standard output. */
	extern FILE *stderr; /* Standard error.  */

#endif /* STDIO_H_ */
