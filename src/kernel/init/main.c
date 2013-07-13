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

int errno = 0;

pid_t wait(int *stat_loc)
{
	pid_t pid;
	
	__asm__ volatile(    \
		"int $0x80"      \
		: "=a" (pid)     \
		: "0" (NR_wait), \
		  "b" (stat_loc) \
	);
	
	if (pid < 0)
	{
		errno = -pid;
		return (-1);
	}
	
	return (pid);
}


PUBLIC void kmain()
{		
	/* Initialize system modules. */
	dev_init();
	mm_init();
	pm_init();
	
	
	while (1)
	kprintf("hi");
}
