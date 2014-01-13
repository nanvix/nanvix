/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * <stdio.h> - Standard I/O library.
 */

#ifndef STDIO_H_
#define STDIO_H_

	#include <sys/types.h>
	#include <limits.h>
	#include <stdarg.h>
	#include <unistd.h>
	
	/* Standard size for a file stream buffer. */
	#define BUFSIZ 128
	
	/* Buffering modes. */
	#define _IOFBF 0 /* Input/output fully buffered. */
	#define _IOLBF 1 /* Input/output line buffered.  */
	#define _IONBF 2 /* Input/output unbuffered.     */
	
	/* End of file. */
	#define EOF -1
	
	/* Maximum number of streams that can be open simultaneously. */
	#define FOPEN_MAX OPEN_MAX

	/*
	 * File stream.
	 */
	typedef struct
	{
		int fd;     /* File descriptor.       */
		off_t size; /* File size.             */
		off_t ptr;  /* File-position cursor   */
		int eof;    /* End of file indicator. */
		int error;  /* Error indicator.       */
		int omode;  /* File open mode.        */
		
		/* I/O buffer. */
		char *buffer; /* The buffer itself. */
		size_t bufsz; /* Buffer size.       */
		int nread;    /* Read pointer.      */
		int nwritten; /* Write pointer.     */
		int buf_mode; /* Buffering mode.    */
		int usr_buf;  /* User buffer?       */
	} FILE;

	extern FILE *fclose(FILE *file);
	
	extern FILE *fopen(const char *name, const char *mode);
	
	extern int fread(void *ptr, size_t size, size_t nmeb, FILE *file);
	
	/*
	 * Writes raw data to a file.
	 */
	extern size_t fwrite(const void *ptr, size_t size, size_t nitems, FILE *stream);
	
	extern int fputc(int c, FILE *file);
	
	extern int fgetc(FILE *file);
	
	extern int ferror(FILE *file);
	
	extern int feof(FILE *file);
	
	/*
	 * Flushes a file stream.
	 */
	extern int fflush(FILE *file);
	
	/*
	 * Fills a file stream.
	 */
	extern int ffill(FILE *stream);
	
	/*
	 * Writes a string to the standard output file.
	 */
	extern int puts(const char *str);
	
	extern int setvbuf(FILE *file, char *buffer, int type, size_t size);
	
	extern int vfscanf(FILE *stream, const char *fmt, va_list args);
	
	extern int fscanf(FILE *stream, const char *fmt, ...);
	
	extern int vfprintf(FILE *stream, const char *fmt, va_list args);
	
	extern int fpritnf(FILE *stream, const char *fmt, ...);
	
	/* Standard file streams. */
	extern FILE *stdin;  /* Standard input.  */
	extern FILE *stdout; /* Standard output. */
	extern FILE *stderr; /* Standard error.  */

#endif /* STDIO_H_ */
