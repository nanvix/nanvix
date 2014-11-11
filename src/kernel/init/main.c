/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * main.c - Kernel main
 */

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <nanvix/mm.h>
#include <nanvix/syscall.h>
#include <fcntl.h>


pid_t fork(void)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_fork)
	);
	
	/* Error. */
	if (pid < 0)
		return (-1);
	
	return (pid);
}

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
	if (ret)
		return (-1);
	
	return (ret);
}

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
 * Opens a file.
 */
int open(const char *path, int oflag, ...)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_open),
		  "b" (path),
		  "c" (oflag),
		  "d" (0)
	);
	
	/* Error. */
	if (ret < 0)
		return (-1);
	
	return (ret);
}

/*
 * Executes init.
 */
PRIVATE void init(void)
{
	const char *argv[] = { "init", "/etc/inittab", NULL };
	const char *envp[] = { "PATH=/bin:/sbin", "HOME=/", NULL };
	
	/* Open standard output streams. */
	open("/dev/tty", O_RDONLY);
	open("/dev/tty", O_WRONLY);
	open("/dev/tty", O_WRONLY);
		
	execve("/sbin/init", argv, envp);
}

/*
 * Initializes the kernel.
 */
PUBLIC void kmain(void)
{		
	pid_t pid;
	
	/* Initialize system modules. */
	dev_init();
	mm_init();
	pm_init();
	fs_init();
	
	pid = fork();
	
	/* Should never occur. */
	if (pid < 0)
		kpanic("failed to fork idle process");
	
	/* init process. */
	else if (pid == 0)
	{	
		init();
		kprintf("failed to execute init");
		_exit(-1);
	}
	
	/* idle process. */	
	while (1)
	{
		halt();
		yield();
	}
}
