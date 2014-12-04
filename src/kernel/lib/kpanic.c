/*
 * Copyright(C) 2011-2014 Pedro H. Penna <pedrohenriquepenna@gmail.com>
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
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 * 
 * You should have received a copy of the GNU General Public License
 * along with Nanvix. If not, see <http://www.gnu.org/licenses/>.
 */

#include <nanvix/const.h>
#include <nanvix/hal.h>
#include <nanvix/klib.h>

/**
 * @brief Writes a message to the kernel's output device and panics the kernel.
 * 
 * @param msg Message to be written onto kernel's output device.
 */
PUBLIC void kpanic(const char *msg)
{
	/*
	 * Disable interrupts, so we cannot
	 * be bothered.
	 */
	disable_interrupts();
	
	kprintf("\nPANIC: %s", msg);
	
	while(1);
		halt();
}
