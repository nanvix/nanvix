/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 *
 * kpanic.c - Kernel panic
 */

#include <asm/util.h>
#include <nanvix/const.h>
#include <nanvix/klib.h>

/*
 * Writes a message to the kernel's output device and panics the kernel.
 */
PUBLIC void kpanic(const char *msg)
{
	disable_interrupts();
	
	kprintf("\nPANIC: %s", msg);
	
	while(1);
		halt();
}
