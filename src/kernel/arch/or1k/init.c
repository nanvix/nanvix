/*
 * Copyright(C) 2011-2018 Pedro H. Penna   <pedrohenriquepenna@gmail.com>
 *              2017-2018 Davidson Francis <davidsondfgl@gmail.com>
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

#include <nanvix/klib.h>
#include <nanvix/syscall.h>

/**
 * @brief Forks the current process.
 * 
 * @returns For the parent process, the process ID of the child process. For
 *          the child process zero is returned. Upon failure, a negative error
 *          code is returned instead.
 */
PRIVATE pid_t fork(void)
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
PRIVATE int execve(const char *filename, const char **argv, const char **envp)
{
	register int ret __asm__("r11") = NR_execve;
	register unsigned r3 __asm__("r3") = (unsigned) filename;
	register unsigned r4 __asm__("r4") = (unsigned) argv;
	register unsigned r5 __asm__("r5") = (unsigned) envp;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (ret)
		: "r" (ret),
		  "r" (r3),
		  "r" (r4),
		  "r" (r5)
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
PRIVATE void _exit(int status)
{
	register unsigned r11 __asm__("r11") = NR__exit;
	register int r3 __asm__("r3") = status;
	
	__asm__ volatile (
		"l.sys 1"
		: "=r" (r11)
		: "r" (r11),
		  "r" (r3)
	);
}

/**
 * @brief Init process.
 */
PUBLIC void init(void)
{
	pid_t pid;
	static int instances = 0;

	if (instances++ > 0)
		kpanic("init process already started");

	/* Spawn init process. */
	if ((pid = fork()) < 0)
		kpanic("failed to fork idle process");
	else if (pid == 0)
	{	
		const char *argv[] = { "init", "/etc/inittab", NULL };
		const char *envp[] = { "PATH=/bin:/sbin", "HOME=/", NULL };
		
		if (execve("/sbin/init", argv, envp))
		{
			kprintf("failed to execute init");
			_exit(-1);	
		}
	}
}
