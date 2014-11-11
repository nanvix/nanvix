/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * hwint.c - Hardware interrupts.
 */

#include <nanvix/const.h>
#include <errno.h>

/*
 * Default hardware interrupt.
 */
PRIVATE void default_hwint() { /* Noop*/ }

/*
 * Hardware interrupt handlers.
 */
PUBLIC void (*hwint_handlers[])(void) = {
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint
};

/*
 * Sets hardware interrupt handler.
 */
PUBLIC int set_hwint(int num, void (*handler)(void))
{
	/* Interrupt handler already set? */
	if (hwint_handlers[num] != &default_hwint)
		return (-EBUSY);
	
	hwint_handlers[num] = handler;
	
	return (0);
}
