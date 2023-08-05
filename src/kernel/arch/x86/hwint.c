/*
 * Copyright(C) 2011-2016 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
#include <nanvix/hal.h>
#include <nanvix/klib.h>
#include <errno.h>

/* Forward definitions. */
PRIVATE void default_hwint(void);

/**
 * @brief Hardware interrupt handlers.
 */
PUBLIC void (*hwint_handlers[])(void) = {
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint,
	&default_hwint, &default_hwint,	&default_hwint, &default_hwint
};

/**
 * @brief Default hardware interrupt handler.
 */
PRIVATE void default_hwint(void)
{
	noop();
}

/**
 * @brief Sets hardware interrupt handler.
 *
 * @param num     Interrupt number.
 * @param handler Interrupt handler.
 *
 * @returns Upon successful completion, zero is returned. Upon failure, a
 *          negative number is returned instead.
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
 * called.
 */
PUBLIC void do_hwint(unsigned irq)
{
	unsigned old_irqlvl;

	old_irqlvl = processor_raise(irq_lvl(irq));

	enable_interrupts();
	hwint_handlers[irq]();
	disable_interrupts();

	processor_drop(old_irqlvl);
}
