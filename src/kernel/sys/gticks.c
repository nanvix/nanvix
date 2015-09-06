/*
 * Copyleft(C) 2015 Davidson Francis <davidsondfgl@gmail.com>
 * 
 * sys/gticks.c - gticks() system call implementation.
 */

#include <nanvix/clock.h>

/*
 * Get ticks since initialization
 */

PUBLIC int sys_gticks()
{
	return ticks;
}