/*
 * Copyright(c) The Maintainers of Nanvix.
 * Licensed under the MIT License.
 */

#ifndef _NANVIX_STDIO_H
#define _NANVIX_STDIO_H

/**
 * @file stdio.h
 * @brief Standard input/output.
 *
 * Declares functions for formatted and unformatted I/O, stream management,
 * and error reporting. Implemented by the libc_stdio Rust crate.
 */

#include <stdarg.h>
#include <stddef.h>
#include <sys/types.h>

#ifdef __cplusplus
extern "C" {
#endif

/*==================================================================================================
 * Constants
 *==================================================================================================*/

#ifndef EOF
#define EOF (-1)
#endif

#ifndef SEEK_SET
#define SEEK_SET 0
#endif

#ifndef SEEK_CUR
#define SEEK_CUR 1
#endif

#ifndef SEEK_END
#define SEEK_END 2
#endif

#ifndef NULL
#define NULL ((void *)0)
#endif

#ifndef BUFSIZ
#define BUFSIZ 8192
#endif

#ifndef FOPEN_MAX
#define FOPEN_MAX 256
#endif

#ifndef FILENAME_MAX
#define FILENAME_MAX 4096
#endif

#ifndef L_tmpnam
#define L_tmpnam 20
#endif

#ifndef TMP_MAX
#define TMP_MAX 10000
#endif

/*==================================================================================================
 * Types
 *==================================================================================================*/

/**
 * @brief Opaque file stream type.
 *
 * The internal layout matches the Rust FILE struct in libc_stdio::streams.
 * Fields should not be accessed directly; use the accessor functions below.
 */
typedef struct {
    int fd;         /**< Underlying file descriptor. */
    int eof;        /**< End-of-file indicator.      */
    int error;      /**< Error indicator.             */
    int ungetc_buf; /**< Push-back character buffer.  */
} FILE;

/*==================================================================================================
 * Standard Streams
 *==================================================================================================*/

extern FILE *stdin(void);
extern FILE *stdout(void);
extern FILE *stderr(void);

/* Convenience macros matching common usage. */
#define stdin (stdin())
#define stdout (stdout())
#define stderr (stderr())

/*==================================================================================================
 * Stream Operations
 *==================================================================================================*/

extern FILE *fopen(const char *pathname, const char *mode);
extern FILE *fdopen(int fd, const char *mode);
extern FILE *freopen(const char *pathname, const char *mode, FILE *stream);
extern int fclose(FILE *stream);
extern int fflush(FILE *stream);
extern void clearerr(FILE *stream);
extern int ferror(FILE *stream);
extern int feof(FILE *stream);
extern int fileno(FILE *stream);
extern int remove(const char *pathname);
extern int rename(const char *old, const char *newpath);
extern FILE *tmpfile(void);
extern int sscanf(const char *s, const char *format, ...);
extern int vsscanf(const char *s, const char *format, __builtin_va_list ap);

/* Stream buffering modes (Nanvix streams are unbuffered). */
#define _IOFBF 0
#define _IOLBF 1
#define _IONBF 2
extern void setbuf(FILE *stream, char *buf);
extern int setvbuf(FILE *stream, char *buf, int mode, size_t size);

/* Process I/O (pipe streams). */
extern FILE *popen(const char *command, const char *type);
extern int pclose(FILE *stream);

/*==================================================================================================
 * Positioning
 *==================================================================================================*/

extern int fseek(FILE *stream, long offset, int whence);
extern int fseeko(FILE *stream, off_t offset, int whence);
extern long ftell(FILE *stream);
extern off_t ftello(FILE *stream);
extern void rewind(FILE *stream);

/*==================================================================================================
 * Character I/O
 *==================================================================================================*/

extern int fgetc(FILE *stream);
extern int getc(FILE *stream);
extern int getchar(void);
extern int fputc(int c, FILE *stream);
extern int putchar(int c);
extern int ungetc(int c, FILE *stream);

/* putc is equivalent to fputc. */
#ifndef putc
#define putc(c, stream) fputc((c), (stream))
#endif

/*==================================================================================================
 * String I/O
 *==================================================================================================*/

extern char *fgets(char *s, int size, FILE *stream);
extern int fputs(const char *s, FILE *stream);
extern int puts(const char *s);
extern ssize_t getdelim(char **lineptr, size_t *n, int delim, FILE *stream);
extern ssize_t getline(char **lineptr, size_t *n, FILE *stream);

/*==================================================================================================
 * Block I/O
 *==================================================================================================*/

extern size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream);
extern size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream);

/*==================================================================================================
 * Formatted I/O
 *==================================================================================================*/

extern int printf(const char *fmt, ...);
extern int fprintf(FILE *stream, const char *fmt, ...);
extern int dprintf(int fd, const char *fmt, ...);
extern int sprintf(char *buf, const char *fmt, ...);
extern int snprintf(char *buf, size_t size, const char *fmt, ...);
extern int asprintf(char **strp, const char *fmt, ...);
extern int vdprintf(int fd, const char *fmt, va_list ap);
extern int vprintf(const char *fmt, va_list ap);
extern int vfprintf(FILE *stream, const char *fmt, va_list ap);
extern int vsprintf(char *buf, const char *fmt, va_list ap);
extern int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
extern int vasprintf(char **strp, const char *fmt, va_list ap);

/*==================================================================================================
 * Error Reporting
 *==================================================================================================*/

extern void perror(const char *s);

/*==================================================================================================
 * Unlocked I/O
 *==================================================================================================*/

extern int getc_unlocked(FILE *stream);
extern int getchar_unlocked(void);
extern int putc_unlocked(int c, FILE *stream);
extern int putchar_unlocked(int c);
extern int fgetc_unlocked(FILE *stream);
extern int fputc_unlocked(int c, FILE *stream);
extern char *fgets_unlocked(char *s, int size, FILE *stream);
extern int fputs_unlocked(const char *s, FILE *stream);
extern size_t fread_unlocked(void *ptr, size_t size, size_t nmemb, FILE *stream);
extern size_t fwrite_unlocked(const void *ptr, size_t size, size_t nmemb, FILE *stream);
extern int feof_unlocked(FILE *stream);
extern int ferror_unlocked(FILE *stream);
extern int fileno_unlocked(FILE *stream);
extern void clearerr_unlocked(FILE *stream);
extern int fflush_unlocked(FILE *stream);

/*==================================================================================================
 * File Locking
 *==================================================================================================*/

extern void flockfile(FILE *stream);
extern int ftrylockfile(FILE *stream);
extern void funlockfile(FILE *stream);

#ifdef __cplusplus
}
#endif

#endif /* _NANVIX_STDIO_H */
