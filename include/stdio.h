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

#ifndef STDIO_H_
#define STDIO_H_

	#include <sys/types.h>
	#include <limits.h>
	#include <stdarg.h>

	/* Standard buffer size. */
	#define BUFSIZ 1024

	/* End of file. */
	#define EOF -1

	/* Maximum number of opened file streams. */
	#define FOPEN_MAX OPEN_MAX

	/* File stream flags. */
	#define _IOFBF    00001 /* Fully buffered?               */
	#define _IOLBF    00002 /* Line buffered?                */
	#define _IONBF    00004 /* Unbuffered?                   */
	#define _IORW     00010 /* Read/Write.                   */
	#define _IOREAD   00020 /* Readable?                     */
	#define _IOWRITE  00040 /* Writable?                     */
	#define _IOAPPEND 00100 /* Append?                       */
	#define _IOEOF    00200 /* End of file reached?          */
	#define _IOERROR  00400 /* Error encountered?            */
	#define _IOMYBUF  01000 /* Library buffer?               */
	#define _IOSYNC   02000 /* Sync file position on append? */

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
		int count;     /* Character count.    */
	} FILE;

	/*
	 * Clears the end-of-file and error indicators for a stream.
	 */
	#define clearerr(stream) \
		((void)(stream->flags &= ~(_IOEOF | _IOERROR)))

	/*
	 * Reads a character from a file.
	 */
	int getc(FILE *stream);

	/*
	 * Reads a character from the standard input file.
	 */
	#define getchar() \
		getc(stdin)

	/*
	 * Reads a string from the standard input file.
	 */
	extern char *gets(char *str);

	/*
	 * Closes a file stream.
	 */
	extern int fclose(FILE *stream);

	/*
	 * Asserts if the end-of-file for a stream is set.
	 */
	#define feof(stream) \
		(stream->flags & _IOEOF)

	/*
	 * Flushes a file stream.
	 */
	extern int fflush(FILE *stream);

	/*
	 * Reads a string from a file.
	 */
	extern char *fgets(char *str, int n, FILE *stream);

	/*
	 * Returns the file descriptor associated with a file stream.
	 */
	#define fileno(stream) \
		(stream->fd)

	/*
	 * Writes a formated string to a file.
	 */
	extern int fprintf(FILE *stream, const char *format, ...);

	/*
	 * Writes a character to a file.
	 */
	extern int fputc(int c, FILE *stream);

	/*
	 * Writes a string to a file.
	 */
	extern int fputs(const char *str, FILE *stream);

	/* Forward definitions. */
	extern FILE *fopen(const char *, const char *);
	extern FILE *freopen(const char *filename, const char *mode, FILE *stream);

	/*
	 * Writes a character to a file.
	 */
	extern int putc(int c, FILE *stream);

	/*
	 * Writes a character to the standard output file.
	 */
	#define putchar(c) \
		putc(c, stdout)

	/*
	 * Writes a string to the standard output file.
	 */
	extern int puts(const char *str);

	/*
	 * Writes a formated string to the standard output file.
	 */
	extern int printf(const char *format, ...);

	/*
	 * Assigns a buffer to a stream.
	 */
	extern int setvbuf(FILE *stream, char *buf, int type, size_t size);

	/*
	 * Writes format output of a stdarg argument list to a file.
	 */
	extern int vfprintf(FILE *stream, const char *format, va_list ap);

	/*
	 * Writes format output of a stdarg argument list to a string.
	 */
	extern int vsprintf(char *string, const char *format, va_list args);

	/* Standard file streams. */
	extern FILE *stdin;  /* Standard input.  */
	extern FILE *stdout; /* Standard output. */
	extern FILE *stderr; /* Standard error.  */

#endif /* STDIO_H_ */
