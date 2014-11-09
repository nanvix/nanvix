/*
 * Copyright (C) 2011-2013 Pedro H. Penna <pedrohenriquepenna@gmail.com>
 * 
 * hwint.c - Hardware interrupts.
 */

#include <i386/i386.h>
#include <asm/util.h>
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

/**
 * @brief Hardware interrupt dispatcher.
 * 
 * @details Dispatches the hardware interrupt to the appropriate interrupt 
 *          handler. To do so, first the processor execution level is raised to
 *          block all interrupts from the same source, and then the specific
 *          interrupt handler routine is called. Finally, after that the
 *          interrupt handler has finished its work, the process level is
 *          restored back.
 * 
 * @param irq Interrupt request number.
 * 
 * @note This function assumes that all interrupts are disable when it is 
 *       called.
 */
PUBLIC void do_hwint(unsigned irq)
{
	processor_raise(int_lvl(irq));
	
	enable_interrupts();
	hwint_handlers[irq]();
	disable_interrupts();
	
	processor_drop();
}
