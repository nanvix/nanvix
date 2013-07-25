/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * main.c - Kernel main
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <nanvix/mm.h>

#include <nanvix/syscall.h>
#include <errno.h>

int errno = 0;

pid_t getpid()
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_getpid)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

pid_t fork()
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_fork)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

pid_t wait(int *stat_loc)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_wait),
		  "b" (stat_loc)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

int pause(void)
{
	int ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_pause)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}
	
	return (ret);
}

pid_t setpgrp(void)
{
	pid_t pid;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (pid)
		: "0" (NR_setpgrp)
	);
	
	/* Error. */
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}

int kill(pid_t pid, int sig)
{
	int ret;

	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_kill),
		  "b" (pid),
		  "c" (sig)
	);
	
	/* Error. */
	if (ret < 0)
	{
		errno = -ret;
		return (-1);
	}

	return (0);
}


sighandler_t signal(int signum, sighandler_t handler, void (*restorer)(void))
{
	sighandler_t old_handler;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (old_handler)
		: "0" (NR_signal),
		  "b" (signum),
		  "c" (handler),
		  "d" (restorer)
	);
	
	/* Error. */
	if (old_handler == NULL)
	{
		errno = -EINVAL;
		return (NULL);
	}

	return (old_handler);
}


void _exit(int status)
{
	__asm__ (
		"int $0x80" 
		: /* empty. */
		: "a" (NR_exit), 
		  "b" (status)
	);
}

unsigned alarm(unsigned seconds)
{
	unsigned ret;
	
	__asm__ volatile (
		"int $0x80"
		: "=a" (ret)
		: "0" (NR_alarm),
		  "b" (seconds)
	);
	
	return (ret);
}



PRIVATE void init()
{
	if (!fork())
	{
		int i;
		
		setpgrp();
		
		for (i = 0; i < 10; i++)
		{
			if (!fork())
			{
				setpgrp();
				goto done;
			}
			
			yield();
		}
		
		yield();
		
		kill(-5, SIGKILL);
		
		while(1)
			yield();
		
done:
		pause();
		
		kprintf("process %d done %d", getpid(), curr_proc->pgrp->pid);
		
		while(1)
			yield();
	}
	
	while (1)
	{	
		while(1)
			yield();
	}
	
	_exit(0);
}

PUBLIC void kmain()
{		
	pid_t pid;
	
	/* Initialize system modules. */
	dev_init();
	mm_init();
	pm_init();
	
	pid = fork();
	
	if (pid < 0)
		kpanic("init fork failed");
	
	/* init process. */
	else if (pid == 0)
	{
		kprintf("spawning init process...");
		
		init();
	}
	
	/* idle process. */	
	while (1)
	{
		halt();
		yield();
	}
}
