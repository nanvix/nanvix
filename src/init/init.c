/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * init.c - System's init process.
 */

#include <sys/types.h>
#include <fcntl.h>
#include <unistd.h>
#include <stdarg.h>

/*
 * Error code.
 */
int errno;

#include <nanvix/syscall.h>

/*
 * Opens a file.
 */
int open(const char *path, int oflag, ...)
{
	int ret;
	mode_t mode;
	va_list arg;
	
	mode = 0;
	
	if (oflag & O_CREAT)
	{
		va_start(arg, oflag);
		mode = va_arg(arg, mode_t)
		va_end(arg);
	}
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_open),
		  "b" (path),
		  "c" (oflag),
		  "d" (mode)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}

/*
 * Closes a file.
 */
int close(int fd)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_close),
		  "b" (fd)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}

/*
 * Terminates the process.
 */
void _exit(int status)
{
	__asm__ volatile(
		"int $0x80"
		: /* empty. */
		: "a" (NR__exit),
		"b" (status)
	);
}
/*
 * Writes to a file.
 */
ssize_t write(int fd, const void *buf, size_t n)
{
	ssize_t ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_write),
		  "b" (fd),
		  "c" (buf),
		  "d" (n)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return ((ssize_t)ret);
}

/*
 * Spawns other system process.
 */
int main(int argc, char **argv)
{
	int stdout;      /* Standard output file descriptor. */
	const char *msg; /* Hello world message.             */
	
	((void)argc);
	((void)argv);
	
	msg = "Hello user world!";
	
	/* Open standard output. */
	if ((stdout = open("/dev/tty", O_RDWR)) < 0)
		return (-1);
	
	write(stdout, msg, 18);
	
	close(stdout);
	
	return (0);
}


/*
 * Entry point of the program.
 */
void _start(int argc, char **argv)
{
	int ret;
	
	((void)argc);
	((void)argv);
	
	ret = main(0, NULL);
	
	_exit(ret);
}
