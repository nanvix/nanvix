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
#include <nanvix/region.h>
#include <nanvix/syscall.h>

#define syscall0(type, name)                                    \
    type name(void)                                                 \
    {                                                               \
        type ret;                                                   \
        __asm__ volatile ("int $0x80" : "=a" (ret) : "0" (NR_##name)); \
        if (ret >= 0)  return ret;                                  \
        return -1;                                                  \
    }


syscall0(pid_t, fork)

PUBLIC void kmain()
{		
	/* Initialize system modules. */
	dev_init();
	initreg();
	pm_init();
	
	while(1)
	{
		yield();
	}
}
