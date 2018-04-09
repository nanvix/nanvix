/*
 * Copyright(C) 2011-2016 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2016-2016 Subhra S. Sarkar <rurtle.coder@gmail.com>
 *              2017-2017 Davidson Francis <davidsondfgl@gmail.com>
 *              2017-2017 Clement Rouquier <clementrouquier@gmail.com>
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

#include <nanvix/const.h>
#include <nanvix/fs.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <nanvix/dev.h>
#include <nanvix/pm.h>
#include <nanvix/mm.h>
#include <nanvix/syscall.h>
#include <nanvix/clock.h>
#include <nanvix/debug.h>
#include <fcntl.h>

/**
 * @brief Forks the current process.
 * 
 * @returns For the parent process, the process ID of the child process. For
 *          the child process zero is returned. Upon failure, a negative error
 *          code is returned instead.
 */
pid_t fork(void)
{
	register pid_t pid
		__asm__("r11") = NR_fork;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (pid)
		: "r"  (pid)
	);
	
	/* Error. */
	if (pid < 0)
		return (-1);
	
	return (pid);
}

/**
 * @brief Executes a program.
 * 
 * @param filename Program to be executed.
 * @param argv     Arguments variables to pass to the program.
 * @param envp     Environment variables to pass to the program.
 * 
 * @returns Upon successful completion, this function shall not return. Upon
 *          failure, it does return with a negative error code.
 */
int execve(const char *filename, const char **argv, const char **envp)
{
	register int ret __asm__("r11") = NR_execve;
	register unsigned r3 __asm__("r3") = (unsigned) filename;
	register unsigned r4 __asm__("r4") = (unsigned) argv;
	register unsigned r5 __asm__("r5") = (unsigned) envp;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (ret)
		: "r" (ret), "r" (r3), "r" (r4), "r" (r5)
	);
	
	/* Error. */
	if (ret)
		return (-1);
	
	return (ret);
}

/**
 * @brief Exits the current process.
 * 
 * @param status Exit status.
 */
void _exit(int status)
{
	register unsigned r11 __asm__("r11") = NR__exit;
	register int r3 __asm__("r3") = status;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (r11)
		: "r" (r11), "r" (r3)
	);
}

/**
 * @brief Init process.
 */
PRIVATE void init(void)
{
	const char *argv[] = { "init", "/etc/inittab", NULL };
	const char *envp[] = { "PATH=/bin:/sbin", "HOME=/", NULL };
		
	execve("/sbin/init", argv, envp);
}

/* External declaration for cpu_init() function. */
extern void cpu_init(void);

/**
 * @brief Required function to use GCC varargs
 */
PUBLIC void abort()
{
}

/**
 * @brief Initializes the kernel.
 *
 * @param cmdline Command line parameters.
 */
PUBLIC void kmain(const char* cmdline)
{		
	pid_t pid;         /* Child process ID. */
	struct process *p; /* Working process.  */
	
	if(!kstrcmp(cmdline,"debug"))
		dbg_init();

	/* Initialize system modules. */
	chkout(DEVID(TTY_MAJOR, 0, CHRDEV));

	cpu_init();
	dev_init();
	mm_init();
	pm_init();
	fs_init();

	dbg_execute();

	/* Spawn init process. */
	if ((pid = fork()) < 0)
		kpanic("failed to fork idle process");
	else if (pid == 0)
	{	
		init();
		kprintf("failed to execute init");
		_exit(-1);
	}
	
	/* idle process. */	
	while (1)
	{
		/* Shutting down.*/
		if (shutting_down)
		{
			/* Bury zombie processes. */
			for (p = FIRST_PROC; p <= LAST_PROC; p++)
			{
				if ((p->state == PROC_ZOMBIE) && (p->father == curr_proc))
					bury(p);
			}
			
			/* Halt system. */
			if (nprocs == 1)
			{	
				kprintf("you may now turn off your computer");
				disable_interrupts();
				while (1)
					halt();
			}
		}
		
		halt();
		yield();
	}
}
