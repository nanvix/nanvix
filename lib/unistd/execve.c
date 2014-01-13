/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * unistd/execve.c - execve() system call.
 */

#include <nanvix/syscall.h>
#include <errno.h>

/*
 * Executes a program.
 */
int execve(const char *filename, const char **argv, const char **envp)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_execve),
		  "b" (filename),
		  "c" (argv),
		  "d" (envp)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}
