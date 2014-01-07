/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * main.c - Kernel main
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/klib.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <nanvix/mm.h>

#include <nanvix/clock.h>
#include <nanvix/syscall.h>
#include <errno.h>

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

/* Init arguments. */
PRIVATE const char *argv[] = { "/etc/inittab", NULL };
PRIVATE const char *envp[] = { "PATH=/usr/bin", NULL };

PUBLIC void kmain(void)
{		
	pid_t pid;
	
	/* Initialize system modules. */
	dev_init();
	mm_init();
	fs_init();
	pm_init();
	
	pid = fork();
	
	/* Should never occur. */
	if (pid < 0)
		kpanic("failed to fork idle process");
	
	/* init process. */
	else if (pid == 0)
	{
		kprintf("kernel: spawning init process");
		
		execve("/etc/init", argv, envp);
		
		kpanic("failed to execute init");
	}
	
	/* idle process. */	
	while (1)
	{
		halt();
		yield();
	}
}
